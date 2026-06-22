use {
    crate::{
        conn::DbConn,
        error::{InternalSnafu, NotFoundSnafu, ValidationSnafu},
        recipe,
        request_context::RequestContext,
    },
    db::{
        id::{BookId, DraftId, MealId, MealRecipeId, RecipeId},
        models::{
            meal::{Meal, MealBuilder, MealDetail},
            meal_recipe::{MealRecipe, MealRecipeBuilder, MealRecipeDetail, parse_multiplier},
        },
        schema::{meal_recipes, meals, recipes},
        slugify,
    },
    diesel::prelude::*,
    diesel_async::{AsyncConnection, RunQueryDsl},
    snafu::prelude::*,
    std::collections::HashMap,
};

// TODO: Paginate.
pub async fn list_all(session: &mut RequestContext) -> crate::Result<Vec<Meal>> {
    let rows = meals::table
        .filter(meals::book_id.eq(session.book_id()?))
        .order(meals::name.asc())
        .load(session.conn())
        .await?;

    Ok(rows)
}

/// Delete a meal by slug within the current book. Meal-recipe rows go via FK
/// cascade.
pub async fn delete(session: &mut RequestContext, slug: &str) -> crate::Result<()> {
    let book_id = session.book_id()?;

    diesel::delete(
        meals::table
            .filter(meals::book_id.eq(book_id))
            .filter(meals::slug.eq(slug)),
    )
    .execute(session.conn())
    .await?;

    Ok(())
}

pub async fn get(session: &mut RequestContext, slug: &str) -> crate::Result<MealDetail> {
    let meal: Meal = meals::table
        .filter(meals::book_id.eq(session.book_id()?))
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
        let slug = slugs.get(&meal_recipe.recipe_id).context(NotFoundSnafu {
            msg: format!("recipe {:?} not found", meal_recipe.recipe_id),
        })?;
        let recipe = recipe::get(session, slug).await?;
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

/// The meal's editable columns, used by both the insert and update paths.
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = meals)]
struct MealUpdate<'a> {
    name: &'a str,
}

/// All columns to create a meal: the create-only ones plus the editable ones.
#[derive(Insertable)]
#[diesel(table_name = meals)]
struct MealNew<'a> {
    book_id: BookId,
    slug: &'a str,
    #[diesel(embed)]
    update: MealUpdate<'a>,
}

/// Insert or update the meal and its recipe rows in one transaction, then
/// return the canonical [`MealDetail`] as [`get`] would.
///
/// Rows are matched by `DraftId`; order comes from `Vec` order. Every query
/// is scoped to the session's book.
pub async fn upsert(
    builder: MealBuilder,
    session: &mut RequestContext,
) -> crate::Result<MealDetail> {
    let book_id = session.book_id()?;
    let name = builder.name.trim().to_string();

    let slug = {
        let conn = session.conn();
        conn.transaction(async |conn| {
            let update = MealUpdate { name: &name };

            let (meal_id, slug): (MealId, String) = match builder.id {
                DraftId::Persisted(id) => diesel::update(
                    meals::table
                        .filter(meals::id.eq(id))
                        .filter(meals::book_id.eq(book_id)),
                )
                .set(&update)
                .returning((meals::id, meals::slug))
                .get_result(conn)
                .await
                .optional()?
                .context(NotFoundSnafu {
                    msg: format!("meal {id:?} not found"),
                })?,

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
                        .await?;
                    (id, slug)
                }
            };

            let mut keep: Vec<MealRecipeId> = Vec::new();
            let mut position = 0i32;
            for row in &builder.recipes {
                // Rows with no recipe chosen are unfinished: skip them.
                if row.is_blank() {
                    continue;
                }

                let recipe_id = resolve_recipe_id(conn, book_id, row.recipe_slug.trim()).await?;
                let record = meal_recipe_record(row, book_id, meal_id, recipe_id, position)?;

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
                    .optional()?
                    .context(NotFoundSnafu {
                        msg: format!("meal recipe {id:?} not found"),
                    })?,

                    DraftId::New(_) => {
                        diesel::insert_into(meal_recipes::table)
                            .values(&record)
                            .returning(meal_recipes::id)
                            .get_result(conn)
                            .await?
                    }
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
            .await?;

            Ok::<_, crate::Error>(slug)
        })
        .await?
    };

    get(session, &slug).await
}

/// Writable columns of `meal_recipes`.
#[derive(Insertable, AsChangeset)]
#[diesel(table_name = meal_recipes)]
struct MealRecipeRecord {
    book_id: BookId,
    meal_id: MealId,
    recipe_id: RecipeId,
    multiplier: f64,
    position: i32,
}

/// The columns to write for one meal-recipe row; `recipe_id` is resolved by the
/// caller from `recipe_slug`, `position` comes from list order.
fn meal_recipe_record(
    row: &MealRecipeBuilder,
    book_id: BookId,
    meal_id: MealId,
    recipe_id: RecipeId,
    position: i32,
) -> crate::Result<MealRecipeRecord> {
    Ok(MealRecipeRecord {
        book_id,
        meal_id,
        recipe_id,
        multiplier: parse_multiplier(&row.multiplier)
            .map_err(|msg| ValidationSnafu { msg }.build())?,
        position,
    })
}

/// Resolve a recipe slug to its id within the book.
async fn resolve_recipe_id(
    conn: &mut DbConn,
    book_id: BookId,
    slug: &str,
) -> crate::Result<RecipeId> {
    recipes::table
        .filter(recipes::book_id.eq(book_id))
        .filter(recipes::slug.eq(slug))
        .select(recipes::id)
        .first(conn)
        .await
        .optional()?
        .context(NotFoundSnafu {
            msg: format!("recipe `{slug}` not found"),
        })
}

/// `base`, or `base-2`, `base-3`, … until unused within the book.
async fn unique_meal_slug(conn: &mut DbConn, book_id: BookId, base: &str) -> crate::Result<String> {
    let mut candidate = base.to_string();
    let mut n: u32 = 2;

    loop {
        let taken: bool = diesel::select(diesel::dsl::exists(
            meals::table
                .filter(meals::book_id.eq(book_id))
                .filter(meals::slug.eq(candidate.as_str())),
        ))
        .get_result(conn)
        .await?;

        if !taken {
            return Ok(candidate);
        }

        candidate = format!("{base}-{n}");
        n = n.checked_add(1).context(InternalSnafu {
            msg: "slug space exhausted",
        })?;
    }
}
