use anyhow::{Context, Result, anyhow};
use sqlx::SqliteConnection;
use std::collections::HashMap;
use std::str::FromStr;
use types::{
    GrocerySection, Ingredient, IngredientUpdate, Meal, MealDetail, MealRecipe, NewMeal, NewRecipe,
    NewShoppingList, NewShoppingListItem, NewStep, Recipe, RecipeDetail, RecipeStep,
    RecipeStepIngredient, ShoppingList, ShoppingListDetail, ShoppingListItem, StepInstruction,
    Unit, UnitKind,
};

pub fn forbidden() -> anyhow::Error {
    anyhow!("forbidden")
}

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
        r#"SELECT id as "id!: i64", position as "position!: i64"
           FROM recipe_steps WHERE recipe_id = ? ORDER BY position"#,
        id,
    )
    .fetch_all(pool)
    .await
    .context("get_recipe steps select")?;

    let mut steps = Vec::with_capacity(step_rows.len());
    for sr in step_rows {
        let instr_rows = sqlx::query!(
            r#"SELECT id as "id!: i64",
                      position as "position!: i64",
                      text as "text!"
               FROM recipe_step_instructions
               WHERE step_id = ? ORDER BY position"#,
            sr.id,
        )
        .fetch_all(pool)
        .await
        .context("get_recipe step instructions select")?;

        let instructions = instr_rows
            .into_iter()
            .map(|r| StepInstruction {
                id: r.id,
                position: r.position,
                text: r.text,
            })
            .collect();

        let ing_rows = sqlx::query!(
            r#"SELECT rsi.id as "id!: i64",
                      rsi.ingredient_id as "ingredient_id!: i64",
                      i.name as "ingredient_name!",
                      rsi.quantity as "quantity: f64",
                      rsi.unit_kind,
                      rsi.unit,
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
                let unit = match (r.unit_kind.as_deref(), r.unit) {
                    (Some(kind_str), Some(text)) => {
                        let kind = UnitKind::from_str(kind_str).unwrap_or(UnitKind::Custom);
                        Some(Unit::new(kind, &text).unwrap_or(Unit::Custom(text)))
                    }
                    _ => None,
                };
                RecipeStepIngredient {
                    id: r.id,
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
            instructions,
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
pub async fn delete_recipe(id: i64) -> Result<()> {
    let pool = crate::db::pool().await;

    let blocking_meals = sqlx::query!(
        r#"SELECT m.name as "name!"
           FROM meal_recipes mr
           JOIN meals m ON m.id = mr.meal_id
           WHERE mr.recipe_id = ?
           ORDER BY m.name"#,
        id,
    )
    .fetch_all(pool)
    .await
    .context("check meal references")?;

    if !blocking_meals.is_empty() {
        let names: Vec<String> = blocking_meals.into_iter().map(|r| r.name).collect();
        return Err(anyhow!(
            "recipe is used by {} meal(s): {}. Remove it from those meals first.",
            names.len(),
            names.join(", "),
        ));
    }

    let affected = sqlx::query!("DELETE FROM recipes WHERE id = ?", id)
        .execute(pool)
        .await
        .context("delete recipe")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("recipe {id} not found"));
    }
    Ok(())
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

struct ConvertedStep {
    instructions: Vec<String>,
    ingredients: Vec<ConvertedIngredient>,
}

fn convert_steps(steps: &[NewStep]) -> Result<Vec<ConvertedStep>> {
    let mut out: Vec<ConvertedStep> = Vec::with_capacity(steps.len());
    for (step_idx, step) in steps.iter().enumerate() {
        let mut ings = Vec::with_capacity(step.ingredients.len());
        for (ing_idx, ing) in step.ingredients.iter().enumerate() {
            let ing_name = ing.ingredient_name.trim();
            if ing_name.is_empty() {
                continue;
            }
            if let Some(q) = ing.quantity
                && (!q.is_finite() || q < 0.0)
            {
                return Err(anyhow!(
                    "step {} ingredient {} ({ing_name}): quantity must be a non-negative number, got {}",
                    step_idx + 1,
                    ing_idx + 1,
                    q,
                ));
            }
            let unit = match ing.unit_kind {
                Some(kind) => Some(Unit::new(kind, &ing.unit).map_err(|e| {
                    anyhow!(
                        "step {} ingredient {} ({ing_name}): {e}",
                        step_idx + 1,
                        ing_idx + 1
                    )
                })?),
                None => None,
            };
            ings.push(ConvertedIngredient {
                name: ing_name.to_string(),
                quantity: ing.quantity,
                unit,
            });
        }

        let instructions = step
            .instructions
            .iter()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();

        out.push(ConvertedStep {
            instructions,
            ingredients: ings,
        });
    }
    Ok(out)
}

async fn insert_steps_into(
    conn: &mut SqliteConnection,
    recipe_id: i64,
    converted_steps: Vec<ConvertedStep>,
) -> Result<()> {
    for (step_idx, step) in converted_steps.into_iter().enumerate() {
        let step_position = step_idx as i64;
        let step_id = sqlx::query!(
            r#"INSERT INTO recipe_steps (recipe_id, position)
               VALUES (?, ?)
               RETURNING id as "id!: i64""#,
            recipe_id,
            step_position,
        )
        .fetch_one(&mut *conn)
        .await
        .context("insert step")?
        .id;

        for (instr_idx, text) in step.instructions.iter().enumerate() {
            let instr_position = instr_idx as i64;
            sqlx::query!(
                r#"INSERT INTO recipe_step_instructions
                   (step_id, position, text)
                   VALUES (?, ?, ?)"#,
                step_id,
                instr_position,
                text,
            )
            .execute(&mut *conn)
            .await
            .context("insert step instruction")?;
        }

        for (ing_idx, ing) in step.ingredients.iter().enumerate() {
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
            let unit_kind = ing.unit.as_ref().map(|u| u.kind().to_string());
            let unit_label = ing.unit.as_ref().map(|u| u.label());
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
    quantity: Option<f64>,
    unit: Option<Unit>,
}
pub async fn list_meals() -> Result<Vec<Meal>> {
    let pool = crate::db::pool().await;
    sqlx::query_as!(
        Meal,
        r#"SELECT id as "id!: i64", user_id as "user_id: i64", name as "name!"
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
        r#"SELECT id as "id!: i64", user_id as "user_id: i64", name as "name!"
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
pub async fn create_meal(input: NewMeal, owner_id: i64) -> Result<i64> {
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
        owner_id,
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

pub async fn update_meal(id: i64, input: NewMeal, actor_id: i64, is_admin: bool) -> Result<()> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("meal name is required"));
    }
    validate_meal_recipes(&input.recipes)?;
    let pool = crate::db::pool().await;
    ensure_meal_writable(pool, id, actor_id, is_admin).await?;

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

pub async fn delete_meal(id: i64, actor_id: i64, is_admin: bool) -> Result<()> {
    let pool = crate::db::pool().await;
    ensure_meal_writable(pool, id, actor_id, is_admin).await?;

    let affected = sqlx::query!("DELETE FROM meals WHERE id = ?", id)
        .execute(pool)
        .await
        .context("delete meal")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("meal {id} not found"));
    }
    Ok(())
}

async fn ensure_meal_writable(
    pool: &sqlx::SqlitePool,
    meal_id: i64,
    actor_id: i64,
    is_admin: bool,
) -> Result<()> {
    let row = sqlx::query!(
        r#"SELECT user_id as "user_id!: i64" FROM meals WHERE id = ?"#,
        meal_id,
    )
    .fetch_optional(pool)
    .await
    .context("ensure_meal_writable select")?
    .ok_or_else(|| anyhow!("meal {meal_id} not found"))?;

    if is_admin || row.user_id == actor_id {
        Ok(())
    } else {
        Err(forbidden())
    }
}
fn validate_meal_recipes(recipes: &[types::NewMealRecipe]) -> Result<()> {
    let mut seen = std::collections::HashSet::with_capacity(recipes.len());
    for (idx, mr) in recipes.iter().enumerate() {
        if !mr.multiplier.is_finite() || mr.multiplier <= 0.0 {
            return Err(anyhow!(
                "recipe {} multiplier must be a positive number, got {}",
                idx + 1,
                mr.multiplier
            ));
        }
        if !seen.insert(mr.recipe_id) {
            return Err(anyhow!(
                "recipe {} (id {}) appears more than once; each recipe can only be added to a meal once",
                idx + 1,
                mr.recipe_id,
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
pub async fn list_ingredient_sections() -> Result<HashMap<i64, Option<GrocerySection>>> {
    let pool = crate::db::pool().await;
    let rows = sqlx::query!(r#"SELECT id as "id!: i64", grocery_section FROM ingredients"#,)
        .fetch_all(pool)
        .await
        .context("list_ingredient_sections select")?;

    let mut out = HashMap::with_capacity(rows.len());
    for r in rows {
        let section = r
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
        out.insert(r.id, section);
    }
    Ok(out)
}

pub async fn list_shopping_lists(owner_id: i64, is_admin: bool) -> Result<Vec<ShoppingList>> {
    let pool = crate::db::pool().await;
    let rows = if is_admin {
        sqlx::query_as!(
            ShoppingList,
            r#"SELECT id as "id!: i64", user_id as "user_id: i64", name as "name!"
               FROM shopping_lists ORDER BY name"#,
        )
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as!(
            ShoppingList,
            r#"SELECT id as "id!: i64", user_id as "user_id: i64", name as "name!"
               FROM shopping_lists WHERE user_id = ? ORDER BY name"#,
            owner_id,
        )
        .fetch_all(pool)
        .await
    };
    rows.context("list_shopping_lists select")
}

pub async fn get_shopping_list(id: i64) -> Result<Option<ShoppingListDetail>> {
    let pool = crate::db::pool().await;
    let Some(list) = sqlx::query_as!(
        ShoppingList,
        r#"SELECT id as "id!: i64", user_id as "user_id: i64", name as "name!"
           FROM shopping_lists WHERE id = ?"#,
        id,
    )
    .fetch_optional(pool)
    .await
    .context("get_shopping_list select")?
    else {
        return Ok(None);
    };

    let rows = sqlx::query!(
        r#"SELECT id as "id!: i64",
                  name as "name!",
                  grocery_section as "grocery_section: String",
                  quantity as "quantity: f64",
                  unit_kind as "unit_kind: String",
                  unit as "unit: String",
                  checked as "checked!: bool",
                  position as "position!: i64"
           FROM shopping_list_items
           WHERE shopping_list_id = ?
           ORDER BY position, id"#,
        id,
    )
    .fetch_all(pool)
    .await
    .context("get_shopping_list items select")?;

    let mut items = Vec::with_capacity(rows.len());
    for r in rows {
        let grocery_section = r
            .grocery_section
            .as_deref()
            .map(GrocerySection::from_str)
            .transpose()
            .with_context(|| {
                format!(
                    "shopping_list_item {} has unknown grocery_section `{}`",
                    r.id,
                    r.grocery_section.as_deref().unwrap_or(""),
                )
            })?;
        let unit = match (r.unit_kind.as_deref(), r.unit) {
            (Some(kind_str), Some(text)) => {
                let kind = UnitKind::from_str(kind_str).unwrap_or(UnitKind::Custom);
                Some(Unit::new(kind, &text).unwrap_or(Unit::Custom(text)))
            }
            _ => None,
        };
        items.push(ShoppingListItem {
            id: r.id,
            name: r.name,
            grocery_section,
            quantity: r.quantity,
            unit,
            checked: r.checked,
            position: r.position,
        });
    }

    Ok(Some(ShoppingListDetail { list, items }))
}

pub async fn create_shopping_list(input: NewShoppingList, owner_id: i64) -> Result<i64> {
    let name = input.name.trim();
    if name.is_empty() {
        return Err(anyhow!("shopping list name is required"));
    }
    validate_shopping_items(&input.items)?;

    let pool = crate::db::pool().await;
    let mut tx = pool.begin().await.context("begin tx")?;
    let list_id = sqlx::query!(
        r#"INSERT INTO shopping_lists (user_id, name) VALUES (?, ?)
           RETURNING id as "id!: i64""#,
        owner_id,
        name,
    )
    .fetch_one(&mut *tx)
    .await
    .context("insert shopping_list")?
    .id;

    for (idx, item) in input.items.iter().enumerate() {
        insert_shopping_item_into(&mut tx, list_id, item, idx as i64).await?;
    }
    tx.commit().await.context("commit tx")?;
    Ok(list_id)
}

pub async fn delete_shopping_list(id: i64, actor_id: i64, is_admin: bool) -> Result<()> {
    let pool = crate::db::pool().await;
    ensure_shopping_list_writable(pool, id, actor_id, is_admin).await?;
    let affected = sqlx::query!("DELETE FROM shopping_lists WHERE id = ?", id)
        .execute(pool)
        .await
        .context("delete shopping_list")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("shopping list {id} not found"));
    }
    Ok(())
}

pub async fn add_shopping_list_item(
    list_id: i64,
    item: NewShoppingListItem,
    actor_id: i64,
    is_admin: bool,
) -> Result<i64> {
    if item.name.trim().is_empty() {
        return Err(anyhow!("item name is required"));
    }
    let pool = crate::db::pool().await;
    ensure_shopping_list_writable(pool, list_id, actor_id, is_admin).await?;

    let mut tx = pool.begin().await.context("begin tx")?;
    let next_pos = sqlx::query!(
        r#"SELECT COALESCE(MAX(position) + 1, 0) as "next!: i64"
           FROM shopping_list_items WHERE shopping_list_id = ?"#,
        list_id,
    )
    .fetch_one(&mut *tx)
    .await
    .context("next item position")?
    .next;
    let new_id = insert_shopping_item_into(&mut tx, list_id, &item, next_pos).await?;
    tx.commit().await.context("commit tx")?;
    Ok(new_id)
}

pub async fn set_shopping_list_item_checked(
    item_id: i64,
    checked: bool,
    actor_id: i64,
    is_admin: bool,
) -> Result<()> {
    let pool = crate::db::pool().await;
    ensure_item_writable(pool, item_id, actor_id, is_admin).await?;
    let affected = sqlx::query!(
        "UPDATE shopping_list_items SET checked = ? WHERE id = ?",
        checked,
        item_id,
    )
    .execute(pool)
    .await
    .context("update shopping_list_item checked")?
    .rows_affected();
    if affected == 0 {
        return Err(anyhow!("shopping list item {item_id} not found"));
    }
    Ok(())
}

pub async fn delete_shopping_list_item(item_id: i64, actor_id: i64, is_admin: bool) -> Result<()> {
    let pool = crate::db::pool().await;
    ensure_item_writable(pool, item_id, actor_id, is_admin).await?;
    let affected = sqlx::query!("DELETE FROM shopping_list_items WHERE id = ?", item_id)
        .execute(pool)
        .await
        .context("delete shopping_list_item")?
        .rows_affected();
    if affected == 0 {
        return Err(anyhow!("shopping list item {item_id} not found"));
    }
    Ok(())
}

async fn insert_shopping_item_into(
    conn: &mut SqliteConnection,
    list_id: i64,
    item: &NewShoppingListItem,
    position: i64,
) -> Result<i64> {
    let name = item.name.trim();
    let section = item.grocery_section.map(|s| s.to_string());
    let unit_kind = item.unit.as_ref().map(|u| u.kind().to_string());
    let unit_label = item.unit.as_ref().map(|u| u.label());
    let id = sqlx::query!(
        r#"INSERT INTO shopping_list_items
           (shopping_list_id, name, grocery_section, quantity, unit_kind, unit, position)
           VALUES (?, ?, ?, ?, ?, ?, ?)
           RETURNING id as "id!: i64""#,
        list_id,
        name,
        section,
        item.quantity,
        unit_kind,
        unit_label,
        position,
    )
    .fetch_one(&mut *conn)
    .await
    .context("insert shopping_list_item")?
    .id;
    Ok(id)
}

fn validate_shopping_items(items: &[NewShoppingListItem]) -> Result<()> {
    for (idx, item) in items.iter().enumerate() {
        if item.name.trim().is_empty() {
            return Err(anyhow!("item {} name is required", idx + 1));
        }
        if let Some(q) = item.quantity
            && (!q.is_finite() || q < 0.0)
        {
            return Err(anyhow!(
                "item {} ({}): quantity must be a non-negative number, got {}",
                idx + 1,
                item.name,
                q,
            ));
        }
    }
    Ok(())
}

async fn ensure_shopping_list_writable(
    pool: &sqlx::SqlitePool,
    list_id: i64,
    actor_id: i64,
    is_admin: bool,
) -> Result<()> {
    let row = sqlx::query!(
        r#"SELECT user_id as "user_id!: i64" FROM shopping_lists WHERE id = ?"#,
        list_id,
    )
    .fetch_optional(pool)
    .await
    .context("ensure_shopping_list_writable select")?
    .ok_or_else(|| anyhow!("shopping list {list_id} not found"))?;

    if is_admin || row.user_id == actor_id {
        Ok(())
    } else {
        Err(forbidden())
    }
}

async fn ensure_item_writable(
    pool: &sqlx::SqlitePool,
    item_id: i64,
    actor_id: i64,
    is_admin: bool,
) -> Result<()> {
    let row = sqlx::query!(
        r#"SELECT sl.user_id as "user_id!: i64"
           FROM shopping_list_items sli
           JOIN shopping_lists sl ON sl.id = sli.shopping_list_id
           WHERE sli.id = ?"#,
        item_id,
    )
    .fetch_optional(pool)
    .await
    .context("ensure_item_writable select")?
    .ok_or_else(|| anyhow!("shopping list item {item_id} not found"))?;

    if is_admin || row.user_id == actor_id {
        Ok(())
    } else {
        Err(forbidden())
    }
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
            quantity: Some(qty),
            unit_kind: Some(unit_kind),
            unit: unit.into(),
        }
    }
    fn single_step_recipe(name: &str, ingredients: Vec<NewStepIngredient>) -> NewRecipe {
        NewRecipe {
            name: name.into(),
            source: None,
            steps: vec![NewStep {
                instructions: vec!["do the thing".into()],
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

    async fn test_user() -> i64 {
        let pool = crate::db::pool().await;
        sqlx::query!(
            r#"INSERT INTO users (oidc_sub, email, name, groups, is_admin)
               VALUES ('test-sub', 'test@example.com', 'tester', '', 1)
               RETURNING id as "id!: i64""#,
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .id
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
                instructions: vec!["Brown the beef.".into()],
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
        assert_eq!(detail.steps[0].instructions.len(), 1);
        assert_eq!(detail.steps[0].instructions[0].text, "Brown the beef.");
        assert_eq!(detail.steps[0].instructions[0].position, 0);
        assert_eq!(detail.steps[0].position, 0);
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "ground beef");
        assert_eq!(ings[0].unit, Some(Unit::Mass(Mass::Lb)));
        assert_eq!(ings[0].quantity, Some(1.0));
        assert_eq!(ings[1].ingredient_name, "onion");
        assert_eq!(ings[1].unit, Some(Unit::Custom("medium".into())));
        assert_eq!(ings[1].quantity, Some(1.0));
    }
    #[tokio::test]
    async fn volume_units_preserve_original() {
        let input = NewRecipe {
            name: "Salt water".into(),
            source: None,
            steps: vec![NewStep {
                instructions: vec!["Mix.".into()],
                ingredients: vec![
                    ing("water", 1.0, UnitKind::Volume, "cup"),
                    ing("kosher salt", 2.0, UnitKind::Volume, "tsp"),
                ],
            }],
        };
        let id = create_recipe(input).await.unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings[0].unit, Some(Unit::Volume(Volume::Cup)));
        assert_eq!(ings[0].quantity, Some(1.0));
        assert_eq!(ings[1].unit, Some(Unit::Volume(Volume::Tsp)));
        assert_eq!(ings[1].quantity, Some(2.0));
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
            Some(Unit::Count(String::new()))
        );
        assert_eq!(detail.steps[0].ingredients[0].quantity, Some(3.0));
        assert_eq!(
            detail.steps[0].ingredients[1].unit,
            Some(Unit::Count("medium".into()))
        );
        assert_eq!(detail.steps[0].ingredients[1].quantity, Some(3.0));
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
    async fn null_quantity_and_unit_roundtrip() {
        let id = create_recipe(single_step_recipe(
            "Taste",
            vec![
                NewStepIngredient {
                    ingredient_name: "salt".into(),
                    quantity: None,
                    unit_kind: None,
                    unit: String::new(),
                },
                NewStepIngredient {
                    ingredient_name: "pepper".into(),
                    quantity: Some(1.0),
                    unit_kind: None,
                    unit: String::new(),
                },
            ],
        ))
        .await
        .unwrap();
        let detail = get_recipe(id).await.unwrap().unwrap();
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings[0].ingredient_name, "salt");
        assert_eq!(ings[0].quantity, None);
        assert_eq!(ings[0].unit, None);
        assert_eq!(ings[1].ingredient_name, "pepper");
        assert_eq!(ings[1].quantity, Some(1.0));
        assert_eq!(ings[1].unit, None);
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
                    instructions: vec!["old step 1".into()],
                    ingredients: vec![ing("beef", 1.0, UnitKind::Mass, "lb")],
                },
                NewStep {
                    instructions: vec!["old step 2".into()],
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
                    instructions: vec!["new single step".into()],
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
        assert_eq!(detail.steps[0].instructions.len(), 1);
        assert_eq!(detail.steps[0].instructions[0].text, "new single step");
        let ings = &detail.steps[0].ingredients;
        assert_eq!(ings.len(), 2);
        assert_eq!(ings[0].ingredient_name, "beef");
        assert_eq!(ings[0].unit, Some(Unit::Mass(Mass::Lb)));
        assert_eq!(ings[0].quantity, Some(2.0));
        assert_eq!(ings[1].ingredient_name, "tomato");
        assert_eq!(ings[1].quantity, Some(3.0));
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
        assert_eq!(detail.steps[0].ingredients[0].quantity, Some(5.0));
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
    async fn multi_instruction_step_roundtrips_in_order() {
        let id = create_recipe(NewRecipe {
            name: "Layered".into(),
            source: None,
            steps: vec![NewStep {
                instructions: vec![
                    "Preheat the oven.".into(),
                    "  ".into(), // blank — should be skipped
                    "Mix the dry ingredients.".into(),
                    "Fold in the wet ingredients.".into(),
                ],
                ingredients: vec![ing("flour", 1.0, UnitKind::Mass, "g")],
            }],
        })
        .await
        .unwrap();

        let detail = get_recipe(id).await.unwrap().unwrap();
        let instructions = &detail.steps[0].instructions;
        assert_eq!(instructions.len(), 3);
        assert_eq!(instructions[0].text, "Preheat the oven.");
        assert_eq!(instructions[0].position, 0);
        assert_eq!(instructions[1].text, "Mix the dry ingredients.");
        assert_eq!(instructions[1].position, 1);
        assert_eq!(instructions[2].text, "Fold in the wet ingredients.");
        assert_eq!(instructions[2].position, 2);
    }

    #[tokio::test]
    async fn create_meal_then_get_roundtrip() {
        let owner = test_user().await;
        let chili = make_recipe("Chili").await;
        let cornbread = make_recipe("Cornbread").await;
        let meal_id = create_meal(
            NewMeal {
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
            },
            owner,
        )
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
        let owner = test_user().await;
        let err = create_meal(
            NewMeal {
                name: "  ".into(),
                recipes: vec![],
            },
            owner,
        )
        .await
        .expect_err("blank name should err");
        assert!(format!("{err:#}").contains("name is required"));
    }

    #[tokio::test]
    async fn create_meal_rejects_non_positive_multiplier() {
        let owner = test_user().await;
        let r = make_recipe("Salt block").await;
        for mult in [0.0_f64, -1.0, f64::NAN] {
            let err = create_meal(
                NewMeal {
                    name: "bad".into(),
                    recipes: vec![NewMealRecipe {
                        recipe_id: r,
                        multiplier: mult,
                    }],
                },
                owner,
            )
            .await
            .expect_err(&format!("multiplier {mult} should err"));
            let msg = format!("{err:#}");
            assert!(msg.contains("positive"), "got: {msg}");
        }
        assert!(list_meals().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_meal_rejects_unknown_recipe_id() {
        let owner = test_user().await;
        let err = create_meal(
            NewMeal {
                name: "x".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: 9999,
                    multiplier: 1.0,
                }],
            },
            owner,
        )
        .await
        .expect_err("missing fk should err");
        assert!(list_meals().await.unwrap().is_empty(), "got err: {err:#}");
    }

    #[tokio::test]
    async fn create_meal_allows_zero_recipes() {
        let owner = test_user().await;
        let id = create_meal(
            NewMeal {
                name: "Empty".into(),
                recipes: vec![],
            },
            owner,
        )
        .await
        .unwrap();
        let detail = get_meal(id).await.unwrap().unwrap();
        assert!(detail.recipes.is_empty());
    }

    #[tokio::test]
    async fn update_meal_replaces_recipes_and_renames() {
        let owner = test_user().await;
        let r1 = make_recipe("A").await;
        let r2 = make_recipe("B").await;
        let r3 = make_recipe("C").await;
        let id = create_meal(
            NewMeal {
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
            },
            owner,
        )
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
            owner,
            false,
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
        let owner = test_user().await;
        let err = update_meal(
            42,
            NewMeal {
                name: "x".into(),
                recipes: vec![],
            },
            owner,
            true,
        )
        .await
        .expect_err("missing id should err");
        assert!(format!("{err:#}").contains("not found"));
    }

    #[tokio::test]
    async fn update_meal_rolls_back_on_unknown_recipe() {
        let owner = test_user().await;
        let r1 = make_recipe("Keeper").await;
        let id = create_meal(
            NewMeal {
                name: "Original".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r1,
                    multiplier: 1.0,
                }],
            },
            owner,
        )
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
            owner,
            false,
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
    async fn update_meal_rejects_non_owner_non_admin() {
        let owner = test_user().await;
        let r = make_recipe("Tomato").await;
        let id = create_meal(
            NewMeal {
                name: "owned".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r,
                    multiplier: 1.0,
                }],
            },
            owner,
        )
        .await
        .unwrap();
        let err = update_meal(
            id,
            NewMeal {
                name: "hijack".into(),
                recipes: vec![],
            },
            owner + 999,
            false,
        )
        .await
        .expect_err("non-owner update should be rejected");
        assert!(format!("{err:#}").contains("forbidden"));
    }

    #[tokio::test]
    async fn delete_meal_admin_can_delete_any() {
        let owner = test_user().await;
        let r = make_recipe("Salt").await;
        let id = create_meal(
            NewMeal {
                name: "Doomed".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r,
                    multiplier: 1.0,
                }],
            },
            owner,
        )
        .await
        .unwrap();
        delete_meal(id, owner + 999, true).await.unwrap();
        assert!(get_meal(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_meal_non_owner_forbidden() {
        let owner = test_user().await;
        let r = make_recipe("Salt").await;
        let id = create_meal(
            NewMeal {
                name: "Mine".into(),
                recipes: vec![NewMealRecipe {
                    recipe_id: r,
                    multiplier: 1.0,
                }],
            },
            owner,
        )
        .await
        .unwrap();
        let err = delete_meal(id, owner + 999, false)
            .await
            .expect_err("non-owner delete should fail");
        assert!(format!("{err:#}").contains("forbidden"));
        assert!(get_meal(id).await.unwrap().is_some());
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
