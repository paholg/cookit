use {
    crate::{conn::DbConn, ingredient, session::Session},
    anyhow::{Context, anyhow},
    db::{
        duration::parse_duration,
        helpers::slugify,
        id::{BookId, DraftId, IngredientId, RecipeId, RecipeStepId, RecipeStepIngredientId},
        models::{
            ingredient::Ingredient,
            recipe::{Recipe, RecipeBuilder, RecipeDetail},
            recipe_step::{RecipeStep, RecipeStepBuilder, RecipeStepDetail},
            recipe_step_ingredient::{
                RecipeStepIngredient, RecipeStepIngredientBuilder, RecipeStepIngredientDetail,
                parse_quantity,
            },
        },
        schema::{ingredients, recipe_step_ingredients, recipe_steps, recipes},
        unit::parse_unit,
    },
    diesel::prelude::*,
    diesel_async::{AsyncConnection, RunQueryDsl},
    std::collections::HashMap,
};

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

pub async fn get(session: &mut Session, slug: &str) -> anyhow::Result<RecipeDetail> {
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

/// The recipe's editable columns, used by both the insert and update paths.
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = recipes)]
struct RecipeUpdate<'a> {
    name: &'a str,
    source: &'a str,
}

/// All columns to create a recipe: the create-only ones plus the editable ones.
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

/// Insert or update the recipe and its steps and ingredients in one
/// transaction, then return the canonical [`RecipeDetail`] as [`get`] would.
///
/// Rows are matched by `DraftId`: new rows are inserted, persisted rows
/// updated, and persisted rows absent from the builder deleted; order comes
/// from the `Vec` order. Every query is scoped to the session's book.
pub async fn upsert(builder: RecipeBuilder, session: &mut Session) -> anyhow::Result<RecipeDetail> {
    let book_id = session.book_id();
    let name = builder.name.trim().to_string();
    let source = builder.source.trim().to_string();

    let slug = {
        let conn = session.conn();
        conn.transaction(async |conn| {
            let update = RecipeUpdate {
                name: &name,
                source: &source,
            };

            let (recipe_id, slug): (RecipeId, String) = match builder.id {
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
            for (idx, step) in builder.steps.iter().enumerate() {
                let record = step_record(step, book_id, recipe_id, idx as i32)?;

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

    get(session, &slug).await
}

/// Writable columns of `recipe_steps`. `treat_none_as_null` makes a cleared
/// timer write SQL `NULL`.
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = recipe_steps, treat_none_as_null = true)]
struct RecipeStepRecord {
    book_id: BookId,
    recipe_id: RecipeId,
    position: i32,
    text: String,
    duration_s: Option<i32>,
}

/// The columns to write for one step; `position` comes from list order.
fn step_record(
    step: &RecipeStepBuilder,
    book_id: BookId,
    recipe_id: RecipeId,
    position: i32,
) -> anyhow::Result<RecipeStepRecord> {
    let duration_s = match step.duration_text.trim() {
        "" => None,
        t => Some(parse_duration(t).map_err(anyhow::Error::msg)? as i32),
    };

    Ok(RecipeStepRecord {
        book_id,
        recipe_id,
        position,
        text: step.instruction.trim().to_string(),
        duration_s,
    })
}

/// Writable columns of `recipe_step_ingredients`. `treat_none_as_null` makes a
/// cleared field write SQL `NULL`.
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = recipe_step_ingredients, treat_none_as_null = true)]
struct RecipeStepIngredientRecord {
    book_id: BookId,
    step_id: RecipeStepId,
    position: i32,
    quantity: Option<f64>,
    unit_kind: Option<String>,
    unit: Option<String>,
    ingredient_id: IngredientId,
}

/// The columns to write for one ingredient row; `ingredient_id` is resolved by
/// the caller, `position` comes from list order.
fn rsi_record(
    ing: &RecipeStepIngredientBuilder,
    book_id: BookId,
    step_id: RecipeStepId,
    position: i32,
    ingredient_id: IngredientId,
) -> anyhow::Result<RecipeStepIngredientRecord> {
    let unit = parse_unit(&ing.unit);

    Ok(RecipeStepIngredientRecord {
        book_id,
        step_id,
        position,
        quantity: parse_quantity(&ing.quantity).map_err(anyhow::Error::msg)?,
        unit_kind: unit.as_ref().map(|u| u.kind().to_string()),
        unit: unit.as_ref().map(|u| u.label()),
        ingredient_id,
    })
}

/// Diff one step's ingredient rows against the database: update persisted rows,
/// insert new ones, prune the rest, assigning positions from list order.
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

        let ingredient_id = ingredient::get_or_create(conn, book_id, ing.name.trim()).await?;
        let record = rsi_record(ing, book_id, step_id, position, ingredient_id)?;

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

/// `base`, or `base-2`, `base-3`, … until unused within the book.
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
