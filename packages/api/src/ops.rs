//! Pool-backed database operations. Each function takes a `&SqlitePool` so it
//! can be exercised by integration tests against a throwaway in-memory database.

use crate::{
    Ingredient, NewRecipe, NewStep, Recipe, RecipeDetail, RecipeStep, RecipeStepIngredient,
    UnitKind, to_canonical,
};
use anyhow::{Context, Result, anyhow};
use sqlx::{Row, SqliteConnection, SqlitePool};

pub async fn list_recipes(pool: &SqlitePool) -> Result<Vec<Recipe>> {
    let rows = sqlx::query("SELECT id, name, source FROM recipes ORDER BY name")
        .fetch_all(pool)
        .await
        .context("list_recipes select")?;
    Ok(rows
        .into_iter()
        .map(|r| Recipe {
            id: r.get("id"),
            name: r.get("name"),
            source: r.get("source"),
        })
        .collect())
}

pub async fn get_recipe(pool: &SqlitePool, id: i64) -> Result<Option<RecipeDetail>> {
    let Some(recipe_row) = sqlx::query("SELECT id, name, source FROM recipes WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("get_recipe select")?
    else {
        return Ok(None);
    };

    let recipe = Recipe {
        id: recipe_row.get("id"),
        name: recipe_row.get("name"),
        source: recipe_row.get("source"),
    };

    let step_rows = sqlx::query(
        "SELECT id, position, instruction FROM recipe_steps \
         WHERE recipe_id = ? ORDER BY position",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .context("get_recipe steps select")?;

    let mut steps = Vec::with_capacity(step_rows.len());
    for sr in step_rows {
        let step_id: i64 = sr.get("id");
        let ing_rows = sqlx::query(
            "SELECT rsi.ingredient_id, i.name AS ingredient_name, rsi.quantity, \
                    rsi.unit_kind, rsi.unit, rsi.position \
             FROM recipe_step_ingredients rsi \
             JOIN ingredients i ON i.id = rsi.ingredient_id \
             WHERE rsi.step_id = ? ORDER BY rsi.position",
        )
        .bind(step_id)
        .fetch_all(pool)
        .await
        .context("get_recipe step ingredients select")?;

        let ingredients = ing_rows
            .into_iter()
            .map(|r| {
                let unit_kind_str: String = r.get("unit_kind");
                RecipeStepIngredient {
                    ingredient_id: r.get("ingredient_id"),
                    ingredient_name: r.get("ingredient_name"),
                    quantity: r.get("quantity"),
                    unit_kind: UnitKind::parse(&unit_kind_str).unwrap_or(UnitKind::Custom),
                    unit: r.get("unit"),
                    position: r.get("position"),
                }
            })
            .collect();

        steps.push(RecipeStep {
            id: step_id,
            position: sr.get("position"),
            instruction: sr.get("instruction"),
            ingredients,
        });
    }

    Ok(Some(RecipeDetail { recipe, steps }))
}

pub async fn list_ingredients(pool: &SqlitePool) -> Result<Vec<Ingredient>> {
    let rows = sqlx::query(
        "SELECT id, name, density_g_per_ml, grocery_section FROM ingredients ORDER BY name",
    )
    .fetch_all(pool)
    .await
    .context("list_ingredients select")?;
    Ok(rows
        .into_iter()
        .map(|r| Ingredient {
            id: r.get("id"),
            name: r.get("name"),
            density_g_per_ml: r.get("density_g_per_ml"),
            grocery_section: r.get("grocery_section"),
        })
        .collect())
}

pub async fn create_recipe(pool: &SqlitePool, input: NewRecipe) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("recipe name is required"));
    }
    let source = input.source.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let converted = convert_steps(&input.steps)?;

    let mut tx = pool.begin().await.context("begin tx")?;

    let recipe_id: i64 =
        sqlx::query("INSERT INTO recipes (name, source) VALUES (?, ?) RETURNING id")
            .bind(name)
            .bind(source)
            .fetch_one(&mut *tx)
            .await
            .context("insert recipe")?
            .get("id");

    insert_steps_into(&mut tx, recipe_id, converted).await?;

    tx.commit().await.context("commit tx")?;
    Ok(recipe_id)
}

pub async fn update_recipe(pool: &SqlitePool, id: i64, input: NewRecipe) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("recipe name is required"));
    }
    let source = input.source.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let converted = convert_steps(&input.steps)?;

    let mut tx = pool.begin().await.context("begin tx")?;

    let affected = sqlx::query("UPDATE recipes SET name = ?, source = ? WHERE id = ?")
        .bind(name)
        .bind(source)
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("update recipe")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("recipe {id} not found"));
    }

    // Steps cascade-delete their step_ingredients, so this clears both.
    sqlx::query("DELETE FROM recipe_steps WHERE recipe_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("delete old steps")?;

    insert_steps_into(&mut tx, id, converted).await?;

    tx.commit().await.context("commit tx")?;
    Ok(())
}

fn convert_steps(steps: &[NewStep]) -> Result<Vec<(String, Vec<ConvertedIngredient>)>> {
    let mut out: Vec<(String, Vec<ConvertedIngredient>)> = Vec::with_capacity(steps.len());
    for (step_idx, step) in steps.iter().enumerate() {
        let mut ings = Vec::with_capacity(step.ingredients.len());
        for (ing_idx, ing) in step.ingredients.iter().enumerate() {
            let ing_name = ing.ingredient_name.trim();
            if ing_name.is_empty() {
                continue;
            }
            let kind = ing.unit_kind.unwrap_or(UnitKind::Custom);
            let (qty, unit) = to_canonical(kind, ing.quantity, &ing.unit).map_err(|e| {
                anyhow!("step {} ingredient {} ({ing_name}): {e}", step_idx + 1, ing_idx + 1)
            })?;
            ings.push(ConvertedIngredient {
                name: ing_name.to_string(),
                quantity: qty,
                unit_kind: kind,
                unit,
            });
        }
        out.push((step.instruction.clone(), ings));
    }
    Ok(out)
}

async fn insert_steps_into(
    conn: &mut SqliteConnection,
    recipe_id: i64,
    converted_steps: Vec<(String, Vec<ConvertedIngredient>)>,
) -> Result<()> {
    for (step_idx, (instruction, ingredients)) in converted_steps.into_iter().enumerate() {
        let step_id: i64 = sqlx::query(
            "INSERT INTO recipe_steps (recipe_id, position, instruction) \
             VALUES (?, ?, ?) RETURNING id",
        )
        .bind(recipe_id)
        .bind(step_idx as i64)
        .bind(&instruction)
        .fetch_one(&mut *conn)
        .await
        .context("insert step")?
        .get("id");

        for (ing_idx, ing) in ingredients.iter().enumerate() {
            let ingredient_id: i64 = match sqlx::query(
                "SELECT id FROM ingredients WHERE name = ? COLLATE NOCASE",
            )
            .bind(&ing.name)
            .fetch_optional(&mut *conn)
            .await
            .context("select ingredient")?
            {
                Some(row) => row.get("id"),
                None => sqlx::query("INSERT INTO ingredients (name) VALUES (?) RETURNING id")
                    .bind(&ing.name)
                    .fetch_one(&mut *conn)
                    .await
                    .context("insert ingredient")?
                    .get("id"),
            };

            sqlx::query(
                "INSERT INTO recipe_step_ingredients \
                 (step_id, ingredient_id, quantity, unit_kind, unit, position) \
                 VALUES (?, ?, ?, ?, ?, ?)",
            )
            .bind(step_id)
            .bind(ingredient_id)
            .bind(ing.quantity)
            .bind(ing.unit_kind.as_str())
            .bind(&ing.unit)
            .bind(ing_idx as i64)
            .execute(&mut *conn)
            .await
            .context("insert step ingredient")?;
        }
    }
    Ok(())
}

struct ConvertedIngredient {
    name: String,
    quantity: f64,
    unit_kind: UnitKind,
    unit: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NewRecipe, NewStep, NewStepIngredient, UnitKind};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::str::FromStr;

    async fn test_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .expect("connect in-memory");
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("migrate");
        pool
    }

    fn ing(name: &str, qty: f64, unit_kind: UnitKind, unit: &str) -> NewStepIngredient {
        NewStepIngredient {
            ingredient_name: name.into(),
            quantity: qty,
            unit_kind: Some(unit_kind),
            unit: unit.into(),
        }
    }

    fn single_step_recipe(name: &str, ingredients: Vec<NewStepIngredient>) -> NewRecipe {
        NewRecipe {
            name: name.into(),
            source: None,
            steps: vec![NewStep {
                instruction: "do the thing".into(),
                ingredients,
            }],
        }
    }

    // ---------- create ----------

    #[tokio::test]
    async fn create_then_get_roundtrip() {
        let pool = test_pool().await;
        let input = NewRecipe {
            name: "Chili".into(),
            source: Some("https://example.com/chili".into()),
            steps: vec![NewStep {
                instruction: "Brown the beef.".into(),
                ingredients: vec![
                    ing("ground beef", 1.0, UnitKind::Mass, "lb"),
                    ing("onion", 1.0, UnitKind::Custom, "medium"),
                ],
            }],
        };

        let id = create_recipe(&pool, input).await.expect("create");
        let detail = get_recipe(&pool, id).await.expect("get").expect("found");

        assert_eq!(detail.recipe.name, "Chili");
        assert_eq!(detail.recipe.source.as_deref(), Some("https://example.com/chili"));
        assert_eq!(detail.steps.len(), 1);
        assert_eq!(detail.steps[0].instruction, "Brown the beef.");
        assert_eq!(detail.steps[0].position, 0);

        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);

        assert_eq!(ings[0].ingredient_name, "ground beef");
        assert_eq!(ings[0].unit_kind, UnitKind::Mass);
        assert_eq!(ings[0].unit, "g");
        assert!((ings[0].quantity - 453.59237).abs() < 1e-6, "got {}", ings[0].quantity);

        assert_eq!(ings[1].ingredient_name, "onion");
        assert_eq!(ings[1].unit_kind, UnitKind::Custom);
        assert_eq!(ings[1].unit, "medium");
        assert_eq!(ings[1].quantity, 1.0);
    }

    #[tokio::test]
    async fn volume_units_convert_to_ml() {
        let pool = test_pool().await;
        let input = NewRecipe {
            name: "Salt water".into(),
            source: None,
            steps: vec![NewStep {
                instruction: "Mix.".into(),
                ingredients: vec![
                    ing("water", 1.0, UnitKind::Volume, "cup"),
                    ing("kosher salt", 2.0, UnitKind::Volume, "tsp"),
                ],
            }],
        };

        let id = create_recipe(&pool, input).await.unwrap();
        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;

        assert_eq!(ings[0].unit, "ml");
        assert!((ings[0].quantity - 236.5882365).abs() < 1e-6);
        assert_eq!(ings[1].unit, "ml");
        assert!((ings[1].quantity - 9.8578431875).abs() < 1e-6);
    }

    #[tokio::test]
    async fn count_unit_clears_unit_string() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            single_step_recipe("Eggs", vec![ing("egg", 3.0, UnitKind::Count, "whatever")]),
        )
        .await
        .unwrap();
        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        assert_eq!(detail.steps[0].ingredients[0].unit, "");
        assert_eq!(detail.steps[0].ingredients[0].quantity, 3.0);
    }

    #[tokio::test]
    async fn unknown_mass_unit_is_rejected() {
        let pool = test_pool().await;
        let err = create_recipe(
            &pool,
            single_step_recipe("Bad", vec![ing("flour", 1.0, UnitKind::Mass, "stones")]),
        )
        .await
        .expect_err("should reject");
        let msg = format!("{err:#}");
        assert!(msg.contains("unknown mass unit"), "got: {msg}");
        assert!(msg.contains("stones"), "got: {msg}");

        let recipes = list_recipes(&pool).await.unwrap();
        assert!(recipes.is_empty(), "transaction must roll back, got: {recipes:?}");
    }

    #[tokio::test]
    async fn empty_name_is_rejected() {
        let pool = test_pool().await;
        let err = create_recipe(&pool, NewRecipe { name: "   ".into(), ..Default::default() })
            .await
            .expect_err("should reject");
        assert!(format!("{err:#}").contains("name is required"));
    }

    #[tokio::test]
    async fn blank_source_stores_null() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            NewRecipe { name: "Plain".into(), source: Some("   ".into()), steps: vec![] },
        )
        .await
        .unwrap();
        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.source, None);
    }

    #[tokio::test]
    async fn ingredient_reused_across_recipes_case_insensitive() {
        let pool = test_pool().await;
        create_recipe(
            &pool,
            single_step_recipe("First", vec![ing("Onion", 1.0, UnitKind::Custom, "medium")]),
        )
        .await
        .unwrap();
        create_recipe(
            &pool,
            single_step_recipe("Second", vec![ing("onion", 2.0, UnitKind::Custom, "medium")]),
        )
        .await
        .unwrap();

        let ingredients = list_ingredients(&pool).await.unwrap();
        assert_eq!(ingredients.len(), 1, "got: {ingredients:?}");
    }

    #[tokio::test]
    async fn empty_ingredient_rows_are_skipped_and_positions_reindex() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            single_step_recipe(
                "Sparse",
                vec![
                    ing("", 1.0, UnitKind::Count, ""),
                    ing("salt", 1.0, UnitKind::Mass, "g"),
                    ing("  ", 1.0, UnitKind::Count, ""),
                    ing("pepper", 1.0, UnitKind::Mass, "g"),
                ],
            ),
        )
        .await
        .unwrap();
        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "salt");
        assert_eq!(ings[0].position, 0);
        assert_eq!(ings[1].ingredient_name, "pepper");
        assert_eq!(ings[1].position, 1);
    }

    #[tokio::test]
    async fn get_recipe_returns_none_for_missing_id() {
        let pool = test_pool().await;
        assert!(get_recipe(&pool, 999).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn negative_quantity_is_rejected() {
        let pool = test_pool().await;
        let err = create_recipe(
            &pool,
            single_step_recipe("Bad", vec![ing("flour", -1.0, UnitKind::Mass, "g")]),
        )
        .await
        .expect_err("should reject");
        assert!(format!("{err:#}").contains("non-negative"));
    }

    // ---------- update ----------

    #[tokio::test]
    async fn update_replaces_steps_and_ingredients() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            NewRecipe {
                name: "Chili v1".into(),
                source: Some("old".into()),
                steps: vec![
                    NewStep {
                        instruction: "old step 1".into(),
                        ingredients: vec![ing("beef", 1.0, UnitKind::Mass, "lb")],
                    },
                    NewStep {
                        instruction: "old step 2".into(),
                        ingredients: vec![ing("water", 1.0, UnitKind::Volume, "cup")],
                    },
                ],
            },
        )
        .await
        .unwrap();

        update_recipe(
            &pool,
            id,
            NewRecipe {
                name: "Chili v2".into(),
                source: Some("https://new".into()),
                steps: vec![NewStep {
                    instruction: "new single step".into(),
                    ingredients: vec![
                        ing("beef", 2.0, UnitKind::Mass, "lb"),
                        ing("tomato", 3.0, UnitKind::Count, ""),
                    ],
                }],
            },
        )
        .await
        .unwrap();

        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Chili v2");
        assert_eq!(detail.recipe.source.as_deref(), Some("https://new"));
        assert_eq!(detail.steps.len(), 1);
        assert_eq!(detail.steps[0].instruction, "new single step");

        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "beef");
        assert!((ings[0].quantity - 907.18474).abs() < 1e-6);
        assert_eq!(ings[1].ingredient_name, "tomato");
        assert_eq!(ings[1].quantity, 3.0);

        // ingredient table should reuse "beef" (case-insensitive), gain tomato, but
        // keep water from the old recipe even though no step references it now.
        let all = list_ingredients(&pool).await.unwrap();
        let names: Vec<&str> = all.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"beef"));
        assert!(names.contains(&"tomato"));
        assert!(names.contains(&"water"));
    }

    #[tokio::test]
    async fn update_unknown_id_errors() {
        let pool = test_pool().await;
        let err = update_recipe(
            &pool,
            42,
            single_step_recipe("X", vec![ing("salt", 1.0, UnitKind::Mass, "g")]),
        )
        .await
        .expect_err("missing id should err");
        assert!(format!("{err:#}").contains("not found"));
    }

    #[tokio::test]
    async fn update_rolls_back_on_bad_unit() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            single_step_recipe("Keep me", vec![ing("salt", 5.0, UnitKind::Mass, "g")]),
        )
        .await
        .unwrap();

        let err = update_recipe(
            &pool,
            id,
            single_step_recipe("Rename attempt", vec![ing("salt", 1.0, UnitKind::Mass, "stones")]),
        )
        .await
        .expect_err("bad unit should reject");
        assert!(format!("{err:#}").contains("unknown mass unit"));

        // Original data must be untouched.
        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Keep me");
        assert_eq!(detail.steps[0].ingredients[0].quantity, 5.0);
    }

    #[tokio::test]
    async fn update_can_shrink_steps_to_zero() {
        let pool = test_pool().await;
        let id = create_recipe(
            &pool,
            single_step_recipe("Has step", vec![ing("salt", 1.0, UnitKind::Mass, "g")]),
        )
        .await
        .unwrap();

        update_recipe(
            &pool,
            id,
            NewRecipe { name: "Empty now".into(), source: None, steps: vec![] },
        )
        .await
        .unwrap();

        let detail = get_recipe(&pool, id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Empty now");
        assert!(detail.steps.is_empty());
    }
}
