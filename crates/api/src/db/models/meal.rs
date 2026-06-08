use {
    crate::{
        db::models::meal_recipe::{MealRecipeBuilder, MealRecipeDetail, MealRecipeError},
        helpers::Name,
        id::{BookId, MealDraftId, MealId, MealRecipeDraftId},
    },
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};
#[cfg(feature = "server")]
use {
    crate::{
        db::{
            conn::DbConn,
            models::{book::Book, meal_recipe::MealRecipe, recipe::RecipeDetail},
            prelude::*,
            schema::{meal_recipes, meals, recipes},
        },
        helpers::slugify,
        id::{DraftId, MealRecipeId, RecipeId},
        session::Session,
    },
    anyhow::{Context, anyhow},
    diesel_async::AsyncConnection,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub struct Meal {
    pub id: MealId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub slug: String,
    pub name: String,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[cfg(feature = "server")]
impl Meal {
    // TODO: Paginate.
    pub async fn list_all(session: &mut Session) -> anyhow::Result<Vec<Meal>> {
        let rows = meals::table
            .filter(meals::book_id.eq(session.book_id()))
            .order(meals::name.asc())
            .load(session.conn())
            .await?;

        Ok(rows)
    }

    /// Delete a meal by slug within the current book. Meal-recipe rows go via FK
    /// cascade.
    pub async fn delete(session: &mut Session, slug: &str) -> anyhow::Result<()> {
        let book_id = session.book_id();

        diesel::delete(
            meals::table
                .filter(meals::book_id.eq(book_id))
                .filter(meals::slug.eq(slug)),
        )
        .execute(session.conn())
        .await
        .context("delete meal")?;

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealDetail {
    pub meal: Meal,
    pub recipes: Vec<MealRecipeDetail>,
}

#[cfg(feature = "server")]
impl MealDetail {
    pub async fn get(session: &mut Session, slug: &str) -> anyhow::Result<Self> {
        let meal: Meal = meals::table
            .filter(meals::book_id.eq(session.book_id()))
            .filter(meals::slug.eq(slug))
            .first(session.conn())
            .await?;

        let rows: Vec<MealRecipe> = MealRecipe::belonging_to(&meal)
            .order(meal_recipes::position.asc())
            .load(session.conn())
            .await?;

        // Map each row's recipe_id to its slug, then build the nested
        // RecipeDetail for each. N+1, but meals hold only a handful of recipes.
        let recipe_ids: Vec<RecipeId> = rows.iter().map(|r| r.recipe_id).collect();
        let slugs: HashMap<RecipeId, String> = recipes::table
            .filter(recipes::id.eq_any(&recipe_ids))
            .select((recipes::id, recipes::slug))
            .load::<(RecipeId, String)>(session.conn())
            .await?
            .into_iter()
            .collect();

        let mut recipes_out = Vec::with_capacity(rows.len());
        for meal_recipe in rows {
            let slug = slugs
                .get(&meal_recipe.recipe_id)
                .ok_or_else(|| anyhow!("recipe {:?} not found", meal_recipe.recipe_id))?;
            let recipe = RecipeDetail::get(session, slug).await?;
            recipes_out.push(MealRecipeDetail {
                meal_recipe,
                recipe,
            });
        }

        Ok(MealDetail {
            meal,
            recipes: recipes_out,
        })
    }
}

/// Edit-form representation of a meal with its recipe rows. Binds the form and
/// is the wire payload for [`MealBuilder::upsert`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealBuilder {
    pub id: MealDraftId,
    pub name: String,
    pub recipes: Vec<MealRecipeBuilder>,
}

/// Validation errors mirroring the builder, keyed by `DraftId`. Empty means
/// valid.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MealError {
    pub name: Option<String>,
    pub recipes: HashMap<MealRecipeDraftId, MealRecipeError>,
}

impl MealError {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.recipes.is_empty()
    }

    /// One-line digest for contexts that can't render the structured tree.
    pub fn summary(&self) -> String {
        let mut msgs = Vec::new();

        if let Some(m) = &self.name {
            msgs.push(format!("name: {m}"));
        }
        for row in self.recipes.values() {
            if let Some(m) = &row.multiplier {
                msgs.push(format!("a recipe: {m}"));
            }
        }

        if msgs.is_empty() {
            "invalid meal".to_string()
        } else {
            msgs.join("; ")
        }
    }
}

impl From<MealDetail> for MealBuilder {
    fn from(detail: MealDetail) -> Self {
        Self {
            id: detail.meal.id.into(),
            name: detail.meal.name,
            recipes: detail.recipes.into_iter().map(Into::into).collect(),
        }
    }
}

impl MealBuilder {
    pub fn new() -> Self {
        Self {
            recipes: vec![MealRecipeBuilder {
                multiplier: "1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Validates own fields, then recurses into each recipe row.
    pub fn validate(&self) -> Result<(), MealError> {
        let mut err = MealError::default();

        if Name::parse(&self.name).is_err() {
            err.name = Some("name is required".to_string());
        }

        for row in &self.recipes {
            if let Err(e) = row.validate() {
                err.recipes.insert(row.id, e);
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }
}

/// The meal's editable columns, used by both the insert and update paths.
#[cfg(feature = "server")]
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = meals)]
struct MealUpdate<'a> {
    name: &'a str,
}

/// All columns to create a meal: the create-only ones plus the editable ones.
#[cfg(feature = "server")]
#[derive(Insertable)]
#[diesel(table_name = meals)]
struct MealNew<'a> {
    book_id: BookId,
    slug: &'a str,
    #[diesel(embed)]
    update: MealUpdate<'a>,
}

#[cfg(feature = "server")]
impl MealBuilder {
    /// Insert or update the meal and its recipe rows in one transaction, then
    /// return the canonical [`MealDetail`] as `get` would.
    ///
    /// Rows are matched by `DraftId`; order comes from `Vec` order. Every query
    /// is scoped to the session's book.
    pub async fn upsert(self, session: &mut Session) -> anyhow::Result<MealDetail> {
        let book_id = session.book_id();
        let name = self.name.trim().to_string();

        let slug = {
            let conn = session.conn();
            conn.transaction(async |conn| {
                let update = MealUpdate { name: &name };

                let (meal_id, slug): (MealId, String) = match self.id {
                    DraftId::Persisted(id) => diesel::update(
                        meals::table
                            .filter(meals::id.eq(id))
                            .filter(meals::book_id.eq(book_id)),
                    )
                    .set(&update)
                    .returning((meals::id, meals::slug))
                    .get_result(conn)
                    .await
                    .optional()
                    .context("update meal")?
                    .ok_or_else(|| anyhow!("meal {id:?} not found"))?,

                    DraftId::New(_) => {
                        let slug = unique_meal_slug(conn, book_id, &slugify(&name)).await?;
                        let id = diesel::insert_into(meals::table)
                            .values(&MealNew {
                                book_id,
                                slug: &slug,
                                update,
                            })
                            .returning(meals::id)
                            .get_result::<MealId>(conn)
                            .await
                            .context("insert meal")?;
                        (id, slug)
                    }
                };

                let mut keep: Vec<MealRecipeId> = Vec::new();
                let mut position = 0i32;
                for row in &self.recipes {
                    // Rows with no recipe chosen are unfinished: skip them.
                    if row.is_blank() {
                        continue;
                    }

                    let recipe_id =
                        resolve_recipe_id(conn, book_id, row.recipe_slug.trim()).await?;
                    let record = row.record(book_id, meal_id, recipe_id, position)?;

                    let mr_id = match row.id {
                        DraftId::Persisted(id) => diesel::update(
                            meal_recipes::table
                                .filter(meal_recipes::id.eq(id))
                                .filter(meal_recipes::meal_id.eq(meal_id)),
                        )
                        .set(&record)
                        .returning(meal_recipes::id)
                        .get_result(conn)
                        .await
                        .optional()
                        .context("update meal recipe")?
                        .ok_or_else(|| anyhow!("meal recipe {id:?} not found"))?,

                        DraftId::New(_) => diesel::insert_into(meal_recipes::table)
                            .values(&record)
                            .returning(meal_recipes::id)
                            .get_result(conn)
                            .await
                            .context("insert meal recipe")?,
                    };

                    keep.push(mr_id);
                    position += 1;
                }

                diesel::delete(
                    meal_recipes::table
                        .filter(meal_recipes::meal_id.eq(meal_id))
                        .filter(meal_recipes::id.ne_all(keep)),
                )
                .execute(conn)
                .await
                .context("prune removed meal recipes")?;

                Ok::<_, anyhow::Error>(slug)
            })
            .await?
        };

        MealDetail::get(session, &slug).await
    }
}

/// Resolve a recipe slug to its id within the book.
#[cfg(feature = "server")]
async fn resolve_recipe_id(
    conn: &mut DbConn,
    book_id: BookId,
    slug: &str,
) -> anyhow::Result<RecipeId> {
    recipes::table
        .filter(recipes::book_id.eq(book_id))
        .filter(recipes::slug.eq(slug))
        .select(recipes::id)
        .first(conn)
        .await
        .optional()
        .context("look up recipe by slug")?
        .ok_or_else(|| anyhow!("recipe `{slug}` not found"))
}

/// `base`, or `base-2`, `base-3`, … until unused within the book.
#[cfg(feature = "server")]
async fn unique_meal_slug(
    conn: &mut DbConn,
    book_id: BookId,
    base: &str,
) -> anyhow::Result<String> {
    let mut candidate = base.to_string();
    let mut n: u32 = 2;

    loop {
        let taken: bool = diesel::select(diesel::dsl::exists(
            meals::table
                .filter(meals::book_id.eq(book_id))
                .filter(meals::slug.eq(candidate.as_str())),
        ))
        .get_result(conn)
        .await
        .context("probe meal slug")?;

        if !taken {
            return Ok(candidate);
        }

        candidate = format!("{base}-{n}");
        n = n
            .checked_add(1)
            .ok_or_else(|| anyhow!("slug space exhausted"))?;
    }
}
