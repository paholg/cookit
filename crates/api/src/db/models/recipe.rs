use {
    crate::{
        db::models::recipe_step::{RecipeStepBuilder, RecipeStepDetail, RecipeStepError},
        helpers::Name,
        id::{BookId, RecipeDraftId, RecipeId, RecipeStepDraftId},
    },
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};
#[cfg(feature = "server")]
use {
    crate::{
        db::{
            conn::DbConn,
            models::{
                book::Book,
                ingredient::{Ingredient, IngredientNew},
                recipe_step::RecipeStep,
                recipe_step_ingredient::{
                    RecipeStepIngredient, RecipeStepIngredientBuilder, RecipeStepIngredientDetail,
                },
            },
            prelude::*,
            schema::{ingredients, recipe_step_ingredients, recipe_steps, recipes},
        },
        helpers::slugify,
        id::{DraftId, IngredientId, RecipeStepId, RecipeStepIngredientId},
        session::Session,
    },
    anyhow::{Context, anyhow},
    diesel_async::AsyncConnection,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Recipe {
    pub id: RecipeId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub slug: String,
    pub name: String,
    pub source: String,
    pub description: String,
    pub notes: String,
}

#[cfg(feature = "server")]
impl Recipe {
    // TODO: Paginate.
    pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<Recipe>> {
        let rows = recipes::table
            .filter(recipes::book_id.eq(session.book_id()))
            .order(recipes::name.asc())
            .load(session.conn())
            .await?;

        Ok(rows)
    }

    /// Delete a recipe by slug within the current book. Steps and ingredients go
    /// via FK cascade.
    pub async fn delete(session: &mut Session, slug: &str) -> anyhow::Result<()> {
        let book_id = session.book_id();

        diesel::delete(
            recipes::table
                .filter(recipes::book_id.eq(book_id))
                .filter(recipes::slug.eq(slug)),
        )
        .execute(session.conn())
        .await
        .context("delete recipe")?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDetail {
    pub recipe: Recipe,
    pub steps: Vec<RecipeStepDetail>,
}

#[cfg(feature = "server")]
impl RecipeDetail {
    pub async fn get(session: &mut Session, slug: &str) -> anyhow::Result<Self> {
        let recipe: Recipe = recipes::table
            .filter(recipes::book_id.eq(session.book_id()))
            .filter(recipes::slug.eq(slug))
            .first(session.conn())
            .await?;

        let steps: Vec<RecipeStep> = RecipeStep::belonging_to(&recipe)
            .order(recipe_steps::position.asc())
            .load(session.conn())
            .await?;

        let rsis: Vec<RecipeStepIngredient> = RecipeStepIngredient::belonging_to(&steps)
            .order((
                recipe_step_ingredients::step_id.asc(),
                recipe_step_ingredients::position.asc(),
            ))
            .load(session.conn())
            .await?;

        let ingredient_ids: Vec<_> = rsis.iter().map(|rsi| rsi.ingredient_id).collect();

        let ingredient_map: HashMap<IngredientId, Ingredient> = ingredients::table
            .filter(ingredients::id.eq_any(&ingredient_ids))
            .load(session.conn())
            .await?
            .into_iter()
            .map(|i: Ingredient| (i.id, i))
            .collect();

        let rsis_by_step = rsis.grouped_by(&steps);

        let steps = steps
            .into_iter()
            .zip(rsis_by_step)
            .map(|(step, step_rsis)| {
                let ingredients = step_rsis
                    .into_iter()
                    .map(|rsi| {
                        let ingredient = ingredient_map[&rsi.ingredient_id].clone();
                        RecipeStepIngredientDetail { rsi, ingredient }
                    })
                    .collect();
                RecipeStepDetail { step, ingredients }
            })
            .collect();

        Ok(RecipeDetail { recipe, steps })
    }
}

/// Edit-form representation of a recipe with its steps and ingredients. Binds
/// the form and is the wire payload for [`RecipeBuilder::upsert`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeBuilder {
    pub id: RecipeDraftId,
    pub name: String,
    pub source: String,
    pub steps: Vec<RecipeStepBuilder>,
}

/// Validation errors mirroring the builder tree, keyed by `DraftId`. Empty means
/// valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeError {
    pub name: Option<String>,
    pub steps: HashMap<RecipeStepDraftId, RecipeStepError>,
}

impl RecipeError {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }

    /// One-line digest for contexts that can't render the structured tree.
    pub fn summary(&self) -> String {
        let mut msgs = Vec::new();

        if let Some(m) = &self.name {
            msgs.push(format!("name: {m}"));
        }
        for step in self.steps.values() {
            if let Some(m) = &step.instruction {
                msgs.push(format!("a step: {m}"));
            }
            if let Some(m) = &step.duration {
                msgs.push(format!("a step timer: {m}"));
            }
            for ing in step.ingredients.values() {
                if let Some(m) = &ing.quantity {
                    msgs.push(format!("an ingredient: {m}"));
                }
            }
        }

        if msgs.is_empty() {
            "invalid recipe".to_string()
        } else {
            msgs.join("; ")
        }
    }
}

impl From<RecipeDetail> for RecipeBuilder {
    fn from(detail: RecipeDetail) -> Self {
        Self {
            id: detail.recipe.id.into(),
            name: detail.recipe.name,
            source: detail.recipe.source,
            steps: detail.steps.into_iter().map(Into::into).collect(),
        }
    }
}

impl RecipeBuilder {
    pub fn new() -> Self {
        Self {
            steps: vec![RecipeStepBuilder::default()],
            ..Default::default()
        }
    }

    /// Validates own fields, then recurses into each step.
    pub fn validate(&self) -> Result<(), RecipeError> {
        let mut err = RecipeError::default();

        if Name::parse(&self.name).is_err() {
            err.name = Some("name is required".to_string());
        }

        for step in &self.steps {
            if let Err(e) = step.validate() {
                err.steps.insert(step.id, e);
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}

/// The recipe's editable columns, used by both the insert and update paths.
#[cfg(feature = "server")]
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = recipes)]
struct RecipeUpdate<'a> {
    name: &'a str,
    source: &'a str,
}

/// All columns to create a recipe: the create-only ones plus the editable ones.
#[cfg(feature = "server")]
#[derive(Insertable)]
#[diesel(table_name = recipes)]
struct RecipeNew<'a> {
    book_id: BookId,
    slug: &'a str,
    description: &'a str,
    notes: &'a str,
    #[diesel(embed)]
    update: RecipeUpdate<'a>,
}

#[cfg(feature = "server")]
impl RecipeBuilder {
    /// Insert or update the recipe and its steps and ingredients in one
    /// transaction, then return the canonical [`RecipeDetail`] as `get` would.
    ///
    /// Rows are matched by `DraftId`: new rows are inserted, persisted rows
    /// updated, and persisted rows absent from the builder deleted; order comes
    /// from the `Vec` order. Every query is scoped to the session's book.
    pub async fn upsert(self, session: &mut Session) -> anyhow::Result<RecipeDetail> {
        let book_id = session.book_id();
        let name = self.name.trim().to_string();
        let source = self.source.trim().to_string();

        let slug = {
            let conn = session.conn();
            conn.transaction(async |conn| {
                let update = RecipeUpdate {
                    name: &name,
                    source: &source,
                };

                let (recipe_id, slug): (RecipeId, String) = match self.id {
                    DraftId::Persisted(id) => diesel::update(
                        recipes::table
                            .filter(recipes::id.eq(id))
                            .filter(recipes::book_id.eq(book_id)),
                    )
                    .set(&update)
                    .returning((recipes::id, recipes::slug))
                    .get_result(conn)
                    .await
                    .optional()
                    .context("update recipe")?
                    .ok_or_else(|| anyhow!("recipe {id:?} not found"))?,

                    DraftId::New(_) => {
                        let slug = unique_recipe_slug(conn, book_id, &slugify(&name)).await?;
                        let id = diesel::insert_into(recipes::table)
                            .values(&RecipeNew {
                                book_id,
                                slug: &slug,
                                description: "",
                                notes: "",
                                update,
                            })
                            .returning(recipes::id)
                            .get_result::<RecipeId>(conn)
                            .await
                            .context("insert recipe")?;
                        (id, slug)
                    }
                };

                let mut keep_steps: Vec<RecipeStepId> = Vec::new();
                for (idx, step) in self.steps.iter().enumerate() {
                    let record = step.record(book_id, recipe_id, idx as i32)?;

                    let step_id = match step.id {
                        DraftId::Persisted(id) => diesel::update(
                            recipe_steps::table
                                .filter(recipe_steps::id.eq(id))
                                .filter(recipe_steps::recipe_id.eq(recipe_id)),
                        )
                        .set(&record)
                        .returning(recipe_steps::id)
                        .get_result(conn)
                        .await
                        .optional()
                        .context("update step")?
                        .ok_or_else(|| anyhow!("step {id:?} not found"))?,

                        DraftId::New(_) => diesel::insert_into(recipe_steps::table)
                            .values(&record)
                            .returning(recipe_steps::id)
                            .get_result(conn)
                            .await
                            .context("insert step")?,
                    };

                    upsert_step_ingredients(conn, book_id, step_id, &step.ingredients).await?;
                    keep_steps.push(step_id);
                }

                // Steps removed in the form (persisted, but no longer present)
                // are pruned; their ingredients go via FK cascade.
                diesel::delete(
                    recipe_steps::table
                        .filter(recipe_steps::recipe_id.eq(recipe_id))
                        .filter(recipe_steps::id.ne_all(keep_steps)),
                )
                .execute(conn)
                .await
                .context("prune removed steps")?;

                Ok::<_, anyhow::Error>(slug)
            })
            .await?
        };

        RecipeDetail::get(session, &slug).await
    }
}

/// Diff one step's ingredient rows against the database: update persisted rows,
/// insert new ones, prune the rest, assigning positions from list order.
#[cfg(feature = "server")]
async fn upsert_step_ingredients(
    conn: &mut DbConn,
    book_id: BookId,
    step_id: RecipeStepId,
    builders: &[RecipeStepIngredientBuilder],
) -> anyhow::Result<()> {
    let mut keep: Vec<RecipeStepIngredientId> = Vec::new();
    let mut position = 0i32;

    for ing in builders {
        // Nameless rows are unfinished, not errors: skip them.
        if ing.is_blank() {
            continue;
        }

        let ingredient_id = get_or_create_ingredient(conn, book_id, ing.name.trim()).await?;
        let record = ing.record(book_id, step_id, position, ingredient_id)?;

        let rsi_id = match ing.id {
            DraftId::Persisted(id) => diesel::update(
                recipe_step_ingredients::table
                    .filter(recipe_step_ingredients::id.eq(id))
                    .filter(recipe_step_ingredients::step_id.eq(step_id)),
            )
            .set(&record)
            .returning(recipe_step_ingredients::id)
            .get_result(conn)
            .await
            .optional()
            .context("update step ingredient")?
            .ok_or_else(|| anyhow!("ingredient row {id:?} not found"))?,

            DraftId::New(_) => diesel::insert_into(recipe_step_ingredients::table)
                .values(&record)
                .returning(recipe_step_ingredients::id)
                .get_result(conn)
                .await
                .context("insert step ingredient")?,
        };

        keep.push(rsi_id);
        position += 1;
    }

    diesel::delete(
        recipe_step_ingredients::table
            .filter(recipe_step_ingredients::step_id.eq(step_id))
            .filter(recipe_step_ingredients::id.ne_all(keep)),
    )
    .execute(conn)
    .await
    .context("prune removed ingredients")?;

    Ok(())
}

/// Find an existing ingredient by case-insensitive name within the book, or
/// create a bare one. Returns its id.
#[cfg(feature = "server")]
async fn get_or_create_ingredient(
    conn: &mut DbConn,
    book_id: BookId,
    name: &str,
) -> anyhow::Result<IngredientId> {
    let existing: Option<IngredientId> = ingredients::table
        .filter(ingredients::book_id.eq(book_id))
        .filter(lower(ingredients::name).eq(name.to_lowercase()))
        .select(ingredients::id)
        .first(conn)
        .await
        .optional()
        .context("lookup ingredient by name")?;

    if let Some(id) = existing {
        return Ok(id);
    }

    diesel::insert_into(ingredients::table)
        .values(&IngredientNew { book_id, name })
        .returning(ingredients::id)
        .get_result(conn)
        .await
        .context("insert ingredient")
}

/// `base`, or `base-2`, `base-3`, … until unused within the book.
#[cfg(feature = "server")]
async fn unique_recipe_slug(
    conn: &mut DbConn,
    book_id: BookId,
    base: &str,
) -> anyhow::Result<String> {
    let mut candidate = base.to_string();
    let mut n: u32 = 2;

    loop {
        let taken: bool = diesel::select(diesel::dsl::exists(
            recipes::table
                .filter(recipes::book_id.eq(book_id))
                .filter(recipes::slug.eq(candidate.as_str())),
        ))
        .get_result(conn)
        .await
        .context("probe recipe slug")?;

        if !taken {
            return Ok(candidate);
        }

        candidate = format!("{base}-{n}");
        n = n
            .checked_add(1)
            .ok_or_else(|| anyhow!("slug space exhausted"))?;
    }
}

#[cfg(feature = "server")]
diesel::define_sql_function! {
    /// SQL `lower()`, for case-insensitive ingredient-name matching.
    fn lower(x: diesel::sql_types::Text) -> diesel::sql_types::Text;
}
