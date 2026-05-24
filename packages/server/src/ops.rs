use anyhow::{Context, Result, anyhow};
use sqlx::SqliteConnection;
use std::str::FromStr;
use types::{
    GrocerySection, Ingredient, IngredientUpdate, Meal, MealDetail, MealRecipe, NewMeal, NewRecipe,
    NewStep, Recipe, RecipeDetail, RecipeStep, RecipeStepIngredient, Unit, UnitKind,
};
const DEFAULT_USER_ID: i64 = 1;
pub async fn list_recipes() -> Result<Vec<Recipe>> {
    let pool = crate::db::pool().await;
    sqlx::query_as!(
        Recipe,
        r#"SELECT id as "id!: i64", name as "name!", source
           FROM recipes ORDER BY name"#,
    )
    .fetch_all(pool)
    .await
    .context("list_recipes select")
}
pub async fn get_recipe(id: i64) -> Result<Option<RecipeDetail>> {
    let pool = crate::db::pool().await;
    let Some(recipe) = sqlx::query_as!(
        Recipe,
        r#"SELECT id as "id!: i64", name as "name!", source
           FROM recipes WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await
    .context("get_recipe select")?
    else {
        return Ok(None);
    };
    let step_rows = sqlx::query!(
        r#"SELECT id as "id!: i64", position as "position!: i64",
                  instruction as "instruction!"
           FROM recipe_steps WHERE recipe_id = ? ORDER BY position"#,
        id,
    )
    .fetch_all(pool)
    .await
    .context("get_recipe steps select")?;
    let mut steps = Vec::with_capacity(step_rows.len());
    for sr in step_rows {
        let ing_rows = sqlx::query!(
            r#"SELECT rsi.ingredient_id as "ingredient_id!: i64",
                      i.name as "ingredient_name!",
                      rsi.quantity as "quantity!: f64",
                      rsi.unit_kind as "unit_kind!",
                      rsi.unit as "unit!",
                      rsi.position as "position!: i64"
               FROM recipe_step_ingredients rsi
               JOIN ingredients i ON i.id = rsi.ingredient_id
               WHERE rsi.step_id = ? ORDER BY rsi.position"#,
            sr.id,
        )
        .fetch_all(pool)
        .await
        .context("get_recipe step ingredients select")?;
        let ingredients = ing_rows
            .into_iter()
            .map(|r| {
                let kind = UnitKind::from_str(&r.unit_kind).unwrap_or(UnitKind::Custom);
                let unit = Unit::new(kind, &r.unit).unwrap_or(Unit::Custom(r.unit));
                RecipeStepIngredient {
                    ingredient_id: r.ingredient_id,
                    ingredient_name: r.ingredient_name,
                    quantity: r.quantity,
                    unit,
                    position: r.position,
                }
            })
            .collect();
        steps.push(RecipeStep {
            id: sr.id,
            position: sr.position,
            instruction: sr.instruction,
            ingredients,
        });
    }
    Ok(Some(RecipeDetail { recipe, steps }))
}
pub async fn list_ingredients() -> Result<Vec<Ingredient>> {
    let pool = crate::db::pool().await;
    let rows = sqlx::query!(
        r#"SELECT id as "id!: i64", name as "name!",
                  density_g_per_ml, grocery_section,
                  ignore_density as "ignore_density!: bool"
           FROM ingredients ORDER BY name"#,
    )
    .fetch_all(pool)
    .await
    .context("list_ingredients select")?;

    rows.into_iter()
        .map(|r| {
            let grocery_section = r
                .grocery_section
                .as_deref()
                .map(GrocerySection::from_str)
                .transpose()
                .with_context(|| {
                    format!(
                        "ingredient {} has unknown grocery_section `{}`",
                        r.id,
                        r.grocery_section.as_deref().unwrap_or(""),
                    )
                })?;
            Ok(Ingredient {
                id: r.id,
                name: r.name,
                density_g_per_ml: r.density_g_per_ml,
                grocery_section,
                ignore_density: r.ignore_density,
            })
        })
        .collect()
}
pub async fn update_ingredient(id: i64, input: IngredientUpdate) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("ingredient name is required"));
    }
    if let Some(d) = input.density_g_per_ml
        && (!d.is_finite() || d <= 0.0)
    {
        return Err(anyhow!("density must be a positive number, got {d}"));
    }
    let section = input.grocery_section.map(|s| s.to_string());
    let pool = crate::db::pool().await;
    let affected = sqlx::query!(
        r#"UPDATE ingredients
           SET name = ?, density_g_per_ml = ?, grocery_section = ?, ignore_density = ?
           WHERE id = ?"#,
        name,
        input.density_g_per_ml,
        section,
        input.ignore_density,
        id,
    )
    .execute(pool)
    .await
    .context("update_ingredient")?
    .rows_affected();
    if affected == 0 {
        return Err(anyhow!("ingredient {id} not found"));
    }
    Ok(())
}
pub async fn create_recipe(input: NewRecipe) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("recipe name is required"));
    }
    let source = input
        .source
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let converted = convert_steps(&input.steps)?;
    let pool = crate::db::pool().await;
    let mut tx = pool.begin().await.context("begin tx")?;
    let recipe_id = sqlx::query!(
        r#"INSERT INTO recipes (name, source) VALUES (?, ?)
           RETURNING id as "id!: i64""#,
        name,
        source,
    )
    .fetch_one(&mut *tx)
    .await
    .context("insert recipe")?
    .id;
    insert_steps_into(&mut tx, recipe_id, converted).await?;
    tx.commit().await.context("commit tx")?;
    Ok(recipe_id)
}
pub async fn update_recipe(id: i64, input: NewRecipe) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("recipe name is required"));
    }
    let source = input
        .source
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let converted = convert_steps(&input.steps)?;
    let pool = crate::db::pool().await;
    let mut tx = pool.begin().await.context("begin tx")?;
    let affected = sqlx::query!(
        "UPDATE recipes SET name = ?, source = ? WHERE id = ?",
        name,
        source,
        id,
    )
    .execute(&mut *tx)
    .await
    .context("update recipe")?
    .rows_affected();
    if affected == 0 {
        return Err(anyhow!("recipe {id} not found"));
    }
    sqlx::query!("DELETE FROM recipe_steps WHERE recipe_id = ?", id)
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
            if !ing.quantity.is_finite() || ing.quantity < 0.0 {
                return Err(anyhow!(
                    "step {} ingredient {} ({ing_name}): quantity must be a non-negative number, got {}",
                    step_idx + 1,
                    ing_idx + 1,
                    ing.quantity,
                ));
            }
            let kind = ing.unit_kind.unwrap_or(UnitKind::Custom);
            let unit = Unit::new(kind, &ing.unit).map_err(|e| {
                anyhow!(
                    "step {} ingredient {} ({ing_name}): {e}",
                    step_idx + 1,
                    ing_idx + 1
                )
            })?;
            ings.push(ConvertedIngredient {
                name: ing_name.to_string(),
                quantity: ing.quantity,
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
        let step_position = step_idx as i64;
        let step_id = sqlx::query!(
            r#"INSERT INTO recipe_steps (recipe_id, position, instruction)
               VALUES (?, ?, ?)
               RETURNING id as "id!: i64""#,
            recipe_id,
            step_position,
            instruction,
        )
        .fetch_one(&mut *conn)
        .await
        .context("insert step")?
        .id;
        for (ing_idx, ing) in ingredients.iter().enumerate() {
            let ingredient_id = match sqlx::query!(
                r#"SELECT id as "id!: i64" FROM ingredients
                   WHERE name = ? COLLATE NOCASE"#,
                ing.name,
            )
            .fetch_optional(&mut *conn)
            .await
            .context("select ingredient")?
            {
                Some(row) => row.id,
                None => {
                    sqlx::query!(
                        r#"INSERT INTO ingredients (name) VALUES (?)
                           RETURNING id as "id!: i64""#,
                        ing.name,
                    )
                    .fetch_one(&mut *conn)
                    .await
                    .context("insert ingredient")?
                    .id
                }
            };
            let unit_kind = ing.unit.kind().to_string();
            let unit_label = ing.unit.label();
            let ing_position = ing_idx as i64;
            sqlx::query!(
                r#"INSERT INTO recipe_step_ingredients
                   (step_id, ingredient_id, quantity, unit_kind, unit, position)
                   VALUES (?, ?, ?, ?, ?, ?)"#,
                step_id,
                ingredient_id,
                ing.quantity,
                unit_kind,
                unit_label,
                ing_position,
            )
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
    unit: Unit,
}
pub async fn list_meals() -> Result<Vec<Meal>> {
    let pool = crate::db::pool().await;
    sqlx::query_as!(
        Meal,
        r#"SELECT id as "id!: i64", name as "name!"
           FROM meals ORDER BY name"#,
    )
    .fetch_all(pool)
    .await
    .context("list_meals select")
}
pub async fn get_meal(id: i64) -> Result<Option<MealDetail>> {
    let pool = crate::db::pool().await;
    let Some(meal) = sqlx::query_as!(
        Meal,
        r#"SELECT id as "id!: i64", name as "name!"
           FROM meals WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await
    .context("get_meal select")?
    else {
        return Ok(None);
    };
    let mr_rows = sqlx::query!(
        r#"SELECT recipe_id as "recipe_id!: i64",
                  multiplier as "multiplier!: f64",
                  position as "position!: i64"
           FROM meal_recipes WHERE meal_id = ? ORDER BY position"#,
        id,
    )
    .fetch_all(pool)
    .await
    .context("get_meal recipes select")?;
    let mut recipes = Vec::with_capacity(mr_rows.len());
    for row in mr_rows {
        let detail = get_recipe(row.recipe_id)
            .await?
            .ok_or_else(|| anyhow!("meal {id} references missing recipe {}", row.recipe_id))?;
        recipes.push(MealRecipe {
            multiplier: row.multiplier,
            position: row.position,
            recipe: detail,
        });
    }
    Ok(Some(MealDetail { meal, recipes }))
}
pub async fn create_meal(input: NewMeal) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_meal_recipes(&input.recipes)?;
    let pool = crate::db::pool().await;
    let mut tx = pool.begin().await.context("begin tx")?;
    let meal_id = sqlx::query!(
        r#"INSERT INTO meals (user_id, name) VALUES (?, ?)
           RETURNING id as "id!: i64""#,
        DEFAULT_USER_ID,
        name,
    )
    .fetch_one(&mut *tx)
    .await
    .context("insert meal")?
    .id;
    insert_meal_recipes_into(&mut tx, meal_id, &input.recipes).await?;
    tx.commit().await.context("commit tx")?;
    Ok(meal_id)
}
pub async fn update_meal(id: i64, input: NewMeal) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_meal_recipes(&input.recipes)?;
    let pool = crate::db::pool().await;
    let mut tx = pool.begin().await.context("begin tx")?;
    let affected = sqlx::query!("UPDATE meals SET name = ? WHERE id = ?", name, id)
        .execute(&mut *tx)
        .await
        .context("update meal")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("meal {id} not found"));
    }
    sqlx::query!("DELETE FROM meal_recipes WHERE meal_id = ?", id)
        .execute(&mut *tx)
        .await
        .context("clear meal recipes")?;
    insert_meal_recipes_into(&mut tx, id, &input.recipes).await?;
    tx.commit().await.context("commit tx")?;
    Ok(())
}
fn validate_meal_recipes(recipes: &[types::NewMealRecipe]) -> Result<()> {
    for (idx, mr) in recipes.iter().enumerate() {
        if !mr.multiplier.is_finite() || mr.multiplier <= 0.0 {
            return Err(anyhow!(
                "recipe {} multiplier must be a positive number, got {}",
                idx + 1,
                mr.multiplier
            ));
        }
    }
    Ok(())
}
async fn insert_meal_recipes_into(
    conn: &mut SqliteConnection,
    meal_id: i64,
    recipes: &[types::NewMealRecipe],
) -> Result<()> {
    for (idx, mr) in recipes.iter().enumerate() {
        let position = idx as i64;
        sqlx::query!(
            r#"INSERT INTO meal_recipes (meal_id, recipe_id, multiplier, position)
               VALUES (?, ?, ?, ?)"#,
            meal_id,
            mr.recipe_id,
            mr.multiplier,
            position,
        )
        .execute(&mut *conn)
        .await
        .with_context(|| format!("insert meal_recipe (recipe_id={})", mr.recipe_id))?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use types::{
        GrocerySection, IngredientUpdate, Mass, NewMeal, NewMealRecipe, NewRecipe, NewStep,
        NewStepIngredient, UnitKind, Volume,
    };
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
    async fn make_recipe(name: &str) -> i64 {
        create_recipe(single_step_recipe(
            name,
            vec![ing("salt", 1.0, UnitKind::Mass, "g")],
        ))
        .await
        .unwrap()
    }
    async fn create_named_ingredient(name: &str) -> i64 {
        let id = create_recipe(single_step_recipe(
            &format!("via-{name}"),
            vec![ing(name, 1.0, UnitKind::Mass, "g")],
        ))
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        detail.steps[0].ingredients[0].ingredient_id
    }
    #[tokio::test]
    async fn create_then_get_roundtrip() {
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
        let id = create_recipe(input).await.expect("create");
        let detail = get_recipe(id).await.expect("get").expect("found");
        assert_eq!(detail.recipe.name, "Chili");
        assert_eq!(
            detail.recipe.source.as_deref(),
            Some("https://example.com/chili")
        );
        assert_eq!(detail.steps.len(), 1);
        assert_eq!(detail.steps[0].instruction, "Brown the beef.");
        assert_eq!(detail.steps[0].position, 0);
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "ground beef");
        assert_eq!(ings[0].unit, Unit::Mass(Mass::Lb));
        assert_eq!(ings[0].quantity, 1.0);
        assert_eq!(ings[1].ingredient_name, "onion");
        assert_eq!(ings[1].unit, Unit::Custom("medium".into()));
        assert_eq!(ings[1].quantity, 1.0);
    }
    #[tokio::test]
    async fn volume_units_preserve_original() {
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
        let id = create_recipe(input).await.unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings[0].unit, Unit::Volume(Volume::Cup));
        assert_eq!(ings[0].quantity, 1.0);
        assert_eq!(ings[1].unit, Unit::Volume(Volume::Tsp));
        assert_eq!(ings[1].quantity, 2.0);
    }
    #[tokio::test]
    async fn count_preserves_unit_text() {
        let id = create_recipe(single_step_recipe(
            "Onion soup",
            vec![
                ing("egg", 3.0, UnitKind::Count, ""),
                ing("onion", 3.0, UnitKind::Count, "medium"),
            ],
        ))
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        assert_eq!(
            detail.steps[0].ingredients[0].unit,
            Unit::Count(String::new())
        );
        assert_eq!(detail.steps[0].ingredients[0].quantity, 3.0);
        assert_eq!(
            detail.steps[0].ingredients[1].unit,
            Unit::Count("medium".into())
        );
        assert_eq!(detail.steps[0].ingredients[1].quantity, 3.0);
    }
    #[tokio::test]
    async fn unknown_mass_unit_is_rejected() {
        let err = create_recipe(single_step_recipe(
            "Bad",
            vec![ing("flour", 1.0, UnitKind::Mass, "stones")],
        ))
        .await
        .expect_err("should reject");
        let msg = format!("{err:#}");
        assert!(msg.contains("unknown mass unit"), "got: {msg}");
        assert!(msg.contains("stones"), "got: {msg}");
        let recipes = list_recipes().await.unwrap();
        assert!(
            recipes.is_empty(),
            "transaction must roll back, got: {recipes:?}"
        );
    }
    #[tokio::test]
    async fn empty_name_is_rejected() {
        let err = create_recipe(NewRecipe {
            name: "   ".into(),
            ..Default::default()
        })
        .await
        .expect_err("should reject");
        assert!(format!("{err:#}").contains("name is required"));
    }
    #[tokio::test]
    async fn blank_source_stores_null() {
        let id = create_recipe(NewRecipe {
            name: "Plain".into(),
            source: Some("   ".into()),
            steps: vec![],
        })
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.source, None);
    }
    #[tokio::test]
    async fn ingredient_reused_across_recipes_case_insensitive() {
        create_recipe(single_step_recipe(
            "First",
            vec![ing("Onion", 1.0, UnitKind::Custom, "medium")],
        ))
        .await
        .unwrap();
        create_recipe(single_step_recipe(
            "Second",
            vec![ing("onion", 2.0, UnitKind::Custom, "medium")],
        ))
        .await
        .unwrap();
        let ingredients = list_ingredients().await.unwrap();
        assert_eq!(ingredients.len(), 1, "got: {ingredients:?}");
    }
    #[tokio::test]
    async fn empty_ingredient_rows_are_skipped_and_positions_reindex() {
        let id = create_recipe(single_step_recipe(
            "Sparse",
            vec![
                ing("", 1.0, UnitKind::Count, ""),
                ing("salt", 1.0, UnitKind::Mass, "g"),
                ing("  ", 1.0, UnitKind::Count, ""),
                ing("pepper", 1.0, UnitKind::Mass, "g"),
            ],
        ))
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "salt");
        assert_eq!(ings[0].position, 0);
        assert_eq!(ings[1].ingredient_name, "pepper");
        assert_eq!(ings[1].position, 1);
    }
    #[tokio::test]
    async fn get_recipe_returns_none_for_missing_id() {
        assert!(get_recipe(999).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn negative_quantity_is_rejected() {
        let err = create_recipe(single_step_recipe(
            "Bad",
            vec![ing("flour", -1.0, UnitKind::Mass, "g")],
        ))
        .await
        .expect_err("should reject");
        assert!(format!("{err:#}").contains("non-negative"));
    }
    #[tokio::test]
    async fn update_replaces_steps_and_ingredients() {
        let id = create_recipe(NewRecipe {
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
        })
        .await
        .unwrap();
        update_recipe(
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
        let detail = get_recipe(id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Chili v2");
        assert_eq!(detail.recipe.source.as_deref(), Some("https://new"));
        assert_eq!(detail.steps.len(), 1);
        assert_eq!(detail.steps[0].instruction, "new single step");
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "beef");
        assert_eq!(ings[0].unit, Unit::Mass(Mass::Lb));
        assert_eq!(ings[0].quantity, 2.0);
        assert_eq!(ings[1].ingredient_name, "tomato");
        assert_eq!(ings[1].quantity, 3.0);
        let all = list_ingredients().await.unwrap();
        let names: Vec<&str> = all.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"beef"));
        assert!(names.contains(&"tomato"));
        assert!(names.contains(&"water"));
    }
    #[tokio::test]
    async fn update_unknown_id_errors() {
        let err = update_recipe(
            42,
            single_step_recipe("X", vec![ing("salt", 1.0, UnitKind::Mass, "g")]),
        )
        .await
        .expect_err("missing id should err");
        assert!(format!("{err:#}").contains("not found"));
    }
    #[tokio::test]
    async fn update_rolls_back_on_bad_unit() {
        let id = create_recipe(single_step_recipe(
            "Keep me",
            vec![ing("salt", 5.0, UnitKind::Mass, "g")],
        ))
        .await
        .unwrap();
        let err = update_recipe(
            id,
            single_step_recipe(
                "Rename attempt",
                vec![ing("salt", 1.0, UnitKind::Mass, "stones")],
            ),
        )
        .await
        .expect_err("bad unit should reject");
        assert!(format!("{err:#}").contains("unknown mass unit"));
        let detail = get_recipe(id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Keep me");
        assert_eq!(detail.steps[0].ingredients[0].quantity, 5.0);
    }
    #[tokio::test]
    async fn update_can_shrink_steps_to_zero() {
        let id = create_recipe(single_step_recipe(
            "Has step",
            vec![ing("salt", 1.0, UnitKind::Mass, "g")],
        ))
        .await
        .unwrap();
        update_recipe(
            id,
            NewRecipe {
                name: "Empty now".into(),
                source: None,
                steps: vec![],
            },
        )
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        assert_eq!(detail.recipe.name, "Empty now");
        assert!(detail.steps.is_empty());
    }
    #[tokio::test]
    async fn create_meal_then_get_roundtrip() {
        let chili = make_recipe("Chili").await;
        let cornbread = make_recipe("Cornbread").await;
        let meal_id = create_meal(NewMeal {
            name: "Friday dinner".into(),
            recipes: vec![
                NewMealRecipe {
                    recipe_id: chili,
                    multiplier: 1.0,
                },
                NewMealRecipe {
                    recipe_id: cornbread,
                    multiplier: 2.0,
                },
            ],
        })
        .await
        .unwrap();
        let detail = get_meal(meal_id).await.unwrap().unwrap();
        assert_eq!(detail.meal.name, "Friday dinner");
        assert_eq!(detail.recipes.len(), 2);
        assert_eq!(detail.recipes[0].recipe.recipe.id, chili);
        assert_eq!(detail.recipes[0].multiplier, 1.0);
        assert_eq!(detail.recipes[0].position, 0);
        assert_eq!(detail.recipes[1].recipe.recipe.id, cornbread);
        assert_eq!(detail.recipes[1].multiplier, 2.0);
        assert_eq!(detail.recipes[1].position, 1);
        assert_eq!(detail.recipes[0].recipe.steps.len(), 1);
        assert_eq!(detail.recipes[0].recipe.steps[0].ingredients.len(), 1);
    }
    #[tokio::test]
    async fn create_meal_rejects_blank_name() {
        let err = create_meal(NewMeal {
            name: "  ".into(),
            recipes: vec![],
        })
        .await
        .expect_err("blank name should err");
        assert!(format!("{err:#}").contains("name is required"));
    }
    #[tokio::test]
    async fn create_meal_rejects_non_positive_multiplier() {
        let r = make_recipe("Salt block").await;
        for mult in [0.0_f64, -1.0, f64::NAN] {
            let err = create_meal(NewMeal {
                name: "bad".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r,
                    multiplier: mult,
                }],
            })
            .await
            .expect_err(&format!("multiplier {mult} should err"));
            let msg = format!("{err:#}");
            assert!(msg.contains("positive"), "got: {msg}");
        }
        assert!(list_meals().await.unwrap().is_empty());
    }
    #[tokio::test]
    async fn create_meal_rejects_unknown_recipe_id() {
        let err = create_meal(NewMeal {
            name: "x".into(),
            recipes: vec![NewMealRecipe {
                recipe_id: 9999,
                multiplier: 1.0,
            }],
        })
        .await
        .expect_err("missing fk should err");
        assert!(list_meals().await.unwrap().is_empty(), "got err: {err:#}");
    }
    #[tokio::test]
    async fn create_meal_allows_zero_recipes() {
        let id = create_meal(NewMeal {
            name: "Empty".into(),
            recipes: vec![],
        })
        .await
        .unwrap();
        let detail = get_meal(id).await.unwrap().unwrap();
        assert!(detail.recipes.is_empty());
    }
    #[tokio::test]
    async fn update_meal_replaces_recipes_and_renames() {
        let r1 = make_recipe("A").await;
        let r2 = make_recipe("B").await;
        let r3 = make_recipe("C").await;
        let id = create_meal(NewMeal {
            name: "v1".into(),
            recipes: vec![
                NewMealRecipe {
                    recipe_id: r1,
                    multiplier: 1.0,
                },
                NewMealRecipe {
                    recipe_id: r2,
                    multiplier: 1.0,
                },
            ],
        })
        .await
        .unwrap();
        update_meal(
            id,
            NewMeal {
                name: "v2".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r3,
                    multiplier: 0.5,
                }],
            },
        )
        .await
        .unwrap();
        let detail = get_meal(id).await.unwrap().unwrap();
        assert_eq!(detail.meal.name, "v2");
        assert_eq!(detail.recipes.len(), 1);
        assert_eq!(detail.recipes[0].recipe.recipe.id, r3);
        assert_eq!(detail.recipes[0].multiplier, 0.5);
    }
    #[tokio::test]
    async fn update_meal_unknown_id_errors() {
        let err = update_meal(
            42,
            NewMeal {
                name: "x".into(),
                recipes: vec![],
            },
        )
        .await
        .expect_err("missing id should err");
        assert!(format!("{err:#}").contains("not found"));
    }
    #[tokio::test]
    async fn update_meal_rolls_back_on_unknown_recipe() {
        let r1 = make_recipe("Keeper").await;
        let id = create_meal(NewMeal {
            name: "Original".into(),
            recipes: vec![NewMealRecipe {
                recipe_id: r1,
                multiplier: 1.0,
            }],
        })
        .await
        .unwrap();
        let err = update_meal(
            id,
            NewMeal {
                name: "Mangle".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: 9999,
                    multiplier: 1.0,
                }],
            },
        )
        .await
        .expect_err("bad fk should err");
        let _ = err;
        let detail = get_meal(id).await.unwrap().unwrap();
        assert_eq!(detail.meal.name, "Original");
        assert_eq!(detail.recipes.len(), 1);
        assert_eq!(detail.recipes[0].recipe.recipe.id, r1);
    }
    #[tokio::test]
    async fn get_meal_returns_none_for_missing_id() {
        assert!(get_meal(999).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn list_ingredients_defaults_ignore_density_false() {
        create_named_ingredient("flour").await;
        let list = list_ingredients().await.unwrap();
        assert_eq!(list.len(), 1);
        assert!(!list[0].ignore_density);
        assert!(list[0].is_incomplete());
    }
    #[tokio::test]
    async fn update_ingredient_sets_fields() {
        let id = create_named_ingredient("olive oil").await;
        update_ingredient(
            id,
            IngredientUpdate {
                name: "olive oil".into(),
                density_g_per_ml: Some(0.91),
                grocery_section: Some(GrocerySection::Pantry),
                ignore_density: false,
            },
        )
        .await
        .unwrap();
        let list = list_ingredients().await.unwrap();
        let i = list.iter().find(|i| i.id == id).unwrap();
        assert_eq!(i.density_g_per_ml, Some(0.91));
        assert_eq!(i.grocery_section, Some(GrocerySection::Pantry));
        assert!(!i.ignore_density);
        assert!(!i.is_incomplete());
    }
    #[tokio::test]
    async fn ignore_density_clears_incomplete_flag_without_density() {
        let id = create_named_ingredient("egg").await;
        update_ingredient(
            id,
            IngredientUpdate {
                name: "egg".into(),
                density_g_per_ml: None,
                grocery_section: Some(GrocerySection::Dairy),
                ignore_density: true,
            },
        )
        .await
        .unwrap();
        let i = list_ingredients()
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.id == id)
            .unwrap();
        assert!(i.density_g_per_ml.is_none());
        assert!(i.ignore_density);
        assert!(!i.is_incomplete());
    }
    #[tokio::test]
    async fn missing_section_still_flagged_even_with_ignore_density() {
        let id = create_named_ingredient("egg").await;
        update_ingredient(
            id,
            IngredientUpdate {
                name: "egg".into(),
                density_g_per_ml: None,
                grocery_section: None,
                ignore_density: true,
            },
        )
        .await
        .unwrap();
        let i = list_ingredients()
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.id == id)
            .unwrap();
        assert!(i.is_incomplete(), "section missing should still flag");
    }
    #[tokio::test]
    async fn update_ingredient_rejects_blank_name() {
        let id = create_named_ingredient("x").await;
        let err = update_ingredient(
            id,
            IngredientUpdate {
                name: "  ".into(),
                ..Default::default()
            },
        )
        .await
        .expect_err("blank name should err");
        assert!(format!("{err:#}").contains("name is required"));
    }
    #[tokio::test]
    async fn update_ingredient_rejects_non_positive_density() {
        let id = create_named_ingredient("x").await;
        for d in [0.0_f64, -1.0, f64::NAN] {
            let err = update_ingredient(
                id,
                IngredientUpdate {
                    name: "x".into(),
                    density_g_per_ml: Some(d),
                    ..Default::default()
                },
            )
            .await
            .expect_err(&format!("density {d} should err"));
            assert!(format!("{err:#}").contains("positive"));
        }
    }
    #[tokio::test]
    async fn update_ingredient_unknown_id_errors() {
        let err = update_ingredient(
            42,
            IngredientUpdate {
                name: "x".into(),
                ..Default::default()
            },
        )
        .await
        .expect_err("missing id should err");
        assert!(format!("{err:#}").contains("not found"));
    }
    #[tokio::test]
    async fn update_ingredient_can_rename() {
        let id = create_named_ingredient("kosher salt").await;
        update_ingredient(
            id,
            IngredientUpdate {
                name: "Diamond Crystal kosher salt".into(),
                density_g_per_ml: Some(0.6),
                grocery_section: Some(GrocerySection::Pantry),
                ignore_density: false,
            },
        )
        .await
        .unwrap();
        let i = list_ingredients()
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.id == id)
            .unwrap();
        assert_eq!(i.name, "Diamond Crystal kosher salt");
    }
}
