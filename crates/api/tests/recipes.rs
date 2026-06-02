//! Integration tests for the recipe and ingredient flows, exercised through the
//! server-side route handlers against the real database. They create rows with
//! unique names and delete them at the end, so they're safe to re-run.
#![cfg(feature = "server")]

use {
    api::{
        RecipeBuilder, RecipeStepBuilder, RecipeStepIngredientBuilder, delete_recipe, get_recipe,
        id::DraftId, list_ingredients, list_recipes, update_ingredient, upsert_recipe,
    },
    std::time::{SystemTime, UNIX_EPOCH},
};

/// A name unique enough to avoid colliding with other rows or test runs.
fn unique(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix} {nanos}")
}

fn ingredient_row(name: &str, quantity: &str, unit: &str) -> RecipeStepIngredientBuilder {
    RecipeStepIngredientBuilder {
        id: DraftId::New(0),
        name: name.to_string(),
        quantity: quantity.to_string(),
        unit: unit.to_string(),
    }
}

#[tokio::test]
async fn recipe_upsert_get_edit_delete() {
    let name = unique("Test Recipe");
    let ingredient_name = unique("test-flour");

    let builder = RecipeBuilder {
        id: DraftId::New(0),
        name: name.clone(),
        source: "https://example.com".to_string(),
        steps: vec![RecipeStepBuilder {
            id: DraftId::New(0),
            instruction: "Mix it all together".to_string(),
            duration_text: "5m".to_string(),
            ingredients: vec![ingredient_row(&ingredient_name, "2", "cup")],
        }],
    };

    // Create.
    let detail = upsert_recipe(builder).await.expect("upsert create");
    assert_eq!(detail.recipe.name, name);
    assert_eq!(detail.recipe.source, "https://example.com");
    assert_eq!(detail.steps.len(), 1);

    let step = &detail.steps[0];
    assert_eq!(step.step.text, "Mix it all together");
    assert_eq!(step.step.duration_s, Some(5 * 60));
    assert_eq!(step.ingredients.len(), 1);
    assert_eq!(step.ingredients[0].rsi.quantity, Some(2.0));
    assert_eq!(step.ingredients[0].rsi.unit.as_deref(), Some("cup"));
    assert_eq!(
        step.ingredients[0].ingredient.name.as_ref(),
        ingredient_name
    );

    let slug = detail.recipe.slug.clone();

    // It shows up in the list and reads back by slug.
    let listed = list_recipes().await.expect("list recipes");
    assert!(listed.iter().any(|r| r.slug == slug));

    let fetched = get_recipe(slug.clone()).await.expect("get recipe");
    assert_eq!(fetched.recipe.name, name);

    // Edit: rename, add a second step, drop the ingredient from the first.
    let mut edit = RecipeBuilder::from(fetched);
    let new_name = unique("Renamed Recipe");
    edit.name = new_name.clone();
    edit.steps[0].ingredients.clear();
    edit.steps.push(RecipeStepBuilder {
        id: DraftId::New(0),
        instruction: "Bake".to_string(),
        duration_text: String::new(),
        ingredients: vec![],
    });

    let updated = upsert_recipe(edit).await.expect("upsert edit");
    assert_eq!(updated.recipe.name, new_name);
    assert_eq!(updated.recipe.slug, slug, "slug is stable across edits");
    assert_eq!(updated.steps.len(), 2);
    assert!(updated.steps[0].ingredients.is_empty());
    assert_eq!(updated.steps[1].step.text, "Bake");

    // Delete.
    delete_recipe(slug.clone()).await.expect("delete recipe");
    let after = list_recipes().await.expect("list after delete");
    assert!(!after.iter().any(|r| r.slug == slug));
}

#[tokio::test]
async fn ingredient_update_round_trips() {
    // Seed an ingredient by creating a throwaway recipe that references it.
    let ingredient_name = unique("test-sugar");
    let builder = RecipeBuilder {
        id: DraftId::New(0),
        name: unique("Sugar Recipe"),
        source: String::new(),
        steps: vec![RecipeStepBuilder {
            id: DraftId::New(0),
            instruction: "Add sugar".to_string(),
            duration_text: String::new(),
            ingredients: vec![ingredient_row(&ingredient_name, "1", "g")],
        }],
    };

    let detail = upsert_recipe(builder).await.expect("upsert");
    let slug = detail.recipe.slug.clone();
    let ingredient_id = detail.steps[0].ingredients[0].ingredient.id;

    // The freshly created ingredient has no density/section yet.
    let before = list_ingredients().await.expect("list ingredients");
    let row = before
        .iter()
        .find(|i| i.id == ingredient_id)
        .expect("ingredient present");
    assert!(row.density_g_per_ml.is_none());

    // Update it.
    use api::{
        IngredientUpdate,
        grocery_section::GrocerySection,
        helpers::{Name, PositiveFloat},
    };
    let update = IngredientUpdate {
        name: Name::parse(&ingredient_name).unwrap(),
        density_g_per_ml: Some(PositiveFloat::parse(0.85).unwrap()),
        grocery_section: Some(GrocerySection::Bakery),
    };
    let updated = update_ingredient(ingredient_id, update)
        .await
        .expect("update ingredient");
    assert_eq!(updated.density_g_per_ml, Some(PositiveFloat(0.85)));
    assert_eq!(updated.grocery_section, Some(GrocerySection::Bakery));

    // Cleanup the recipe (ingredient row stays, harmless).
    delete_recipe(slug).await.expect("delete recipe");
}
