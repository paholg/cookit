//! Integration tests for the meal flow, exercised through the server-side route
//! handlers against the real database. Each test creates a recipe to reference,
//! then cleans up after itself.
#![cfg(feature = "server")]

use {
    api::{
        MealBuilder, MealRecipeBuilder, RecipeBuilder, RecipeStepBuilder, delete_meal,
        delete_recipe, get_meal, id::DraftId, list_meals, upsert_meal, upsert_recipe,
    },
    std::time::{SystemTime, UNIX_EPOCH},
};

fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix} {nanos}")
}

/// Create a bare recipe and return its slug.
async fn make_recipe() -> String {
    let builder = RecipeBuilder {
        id: DraftId::New(0),
        name: unique("Meal Test Recipe"),
        source: String::new(),
        steps: vec![RecipeStepBuilder {
            id: DraftId::New(0),
            instruction: "Do the thing".to_string(),
            duration_text: String::new(),
            ingredients: vec![],
        }],
    };
    upsert_recipe(builder)
        .await
        .expect("create recipe")
        .recipe
        .slug
}

#[tokio::test]
async fn meal_upsert_get_edit_delete() {
    let recipe_slug = make_recipe().await;
    let name = unique("Test Meal");

    let builder = MealBuilder {
        id: DraftId::New(0),
        name: name.clone(),
        recipes: vec![MealRecipeBuilder {
            id: DraftId::New(0),
            recipe_slug: recipe_slug.clone(),
            multiplier: "2".to_string(),
        }],
    };

    // Create.
    let detail = upsert_meal(builder).await.expect("upsert create");
    assert_eq!(detail.meal.name, name);
    assert_eq!(detail.recipes.len(), 1);
    assert_eq!(detail.recipes[0].meal_recipe.multiplier, 2.0);
    assert_eq!(detail.recipes[0].recipe.recipe.slug, recipe_slug);

    let slug = detail.meal.slug.clone();

    // Shows up in list and reads back.
    let listed = list_meals().await.expect("list meals");
    assert!(listed.iter().any(|m| m.slug == slug));

    let fetched = get_meal(slug.clone()).await.expect("get meal");
    assert_eq!(fetched.meal.name, name);

    // Edit: rename and change the multiplier.
    let mut edit = MealBuilder::from(fetched);
    let new_name = unique("Renamed Meal");
    edit.name = new_name.clone();
    edit.recipes[0].multiplier = "0.5".to_string();

    let updated = upsert_meal(edit).await.expect("upsert edit");
    assert_eq!(updated.meal.name, new_name);
    assert_eq!(updated.meal.slug, slug, "slug is stable across edits");
    assert_eq!(updated.recipes[0].meal_recipe.multiplier, 0.5);

    // Delete.
    delete_meal(slug.clone()).await.expect("delete meal");
    let after = list_meals().await.expect("list after delete");
    assert!(!after.iter().any(|m| m.slug == slug));

    delete_recipe(recipe_slug).await.expect("cleanup recipe");
}

#[tokio::test]
async fn blank_recipe_rows_are_dropped() {
    let recipe_slug = make_recipe().await;

    let builder = MealBuilder {
        id: DraftId::New(0),
        name: unique("Sparse Meal"),
        recipes: vec![
            MealRecipeBuilder {
                id: DraftId::New(0),
                recipe_slug: recipe_slug.clone(),
                multiplier: String::new(), // empty -> defaults to 1
            },
            MealRecipeBuilder {
                id: DraftId::New(1),
                recipe_slug: String::new(), // blank -> dropped
                multiplier: "3".to_string(),
            },
        ],
    };

    let detail = upsert_meal(builder).await.expect("upsert");
    assert_eq!(detail.recipes.len(), 1, "blank row dropped");
    assert_eq!(detail.recipes[0].meal_recipe.multiplier, 1.0, "empty multiplier defaults to 1");

    delete_meal(detail.meal.slug).await.expect("cleanup meal");
    delete_recipe(recipe_slug).await.expect("cleanup recipe");
}
