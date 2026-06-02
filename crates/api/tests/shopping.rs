use {
    crate::test_support::{TestBook, unique},
    api::{
        MealBuilder, MealRecipeBuilder, RecipeBuilder, RecipeStepBuilder,
        RecipeStepIngredientBuilder, ShoppingListItemInput, add_shopping_list_item,
        create_shopping_list, create_shopping_list_from_meal, delete_meal, delete_recipe,
        delete_shopping_list, delete_shopping_list_item, get_shopping_list, id::DraftId,
        list_shopping_lists, set_shopping_list_item_checked, upsert_meal, upsert_recipe,
    },
};

mod test_support;

fn ing(name: &str, qty: &str, unit: &str) -> RecipeStepIngredientBuilder {
    RecipeStepIngredientBuilder {
        id: DraftId::New(0),
        name: name.to_string(),
        quantity: qty.to_string(),
        unit: unit.to_string(),
    }
}

#[tokio::test]
async fn shopping_list_from_meal_aggregates_and_edits() {
    TestBook::new().await;

    let flour = unique("agg-flour");
    let sugar = unique("agg-sugar");

    // A recipe with two ingredients.
    let recipe = RecipeBuilder {
        id: DraftId::New(0),
        name: unique("Shopping Recipe"),
        source: String::new(),
        steps: vec![RecipeStepBuilder {
            id: DraftId::New(0),
            instruction: "Combine".to_string(),
            duration_text: String::new(),
            ingredients: vec![ing(&flour, "2", "cup"), ing(&sugar, "1", "cup")],
        }],
    };
    let recipe_slug = upsert_recipe(recipe).await.expect("recipe").recipe.slug;

    // A meal that uses it twice over (multiplier 2).
    let meal = MealBuilder {
        id: DraftId::New(0),
        name: unique("Shopping Meal"),
        recipes: vec![MealRecipeBuilder {
            id: DraftId::New(0),
            recipe_slug: recipe_slug.clone(),
            multiplier: "2".to_string(),
        }],
    };
    let meal_slug = upsert_meal(meal).await.expect("meal").meal.slug;

    // Generate a shopping list from the meal.
    let list_id = create_shopping_list_from_meal(meal_slug.clone())
        .await
        .expect("from meal");

    let detail = get_shopping_list(list_id).await.expect("get list");
    assert_eq!(detail.items.len(), 2, "two aggregated ingredients");

    let flour_item = detail
        .items
        .iter()
        .find(|i| i.ingredient_name.as_deref() == Some(flour.as_str()))
        .expect("flour present");
    assert_eq!(flour_item.quantity, Some(4.0), "2 cup x2 multiplier");
    assert_eq!(flour_item.unit.as_deref(), Some("cup"));
    assert!(!flour_item.checked);

    // Add a manual item.
    add_shopping_list_item(
        list_id,
        ShoppingListItemInput {
            text: "Paper towels".to_string(),
            quantity: String::new(),
            unit: String::new(),
        },
    )
    .await
    .expect("add item");

    let detail = get_shopping_list(list_id).await.expect("get list 2");
    assert_eq!(detail.items.len(), 3);

    // Check one item off.
    let first_id = detail.items[0].id;
    set_shopping_list_item_checked(first_id, true)
        .await
        .expect("check");
    let detail = get_shopping_list(list_id).await.expect("get list 3");
    assert_eq!(detail.items.iter().filter(|i| i.checked).count(), 1);

    // Delete the manual item.
    let towel_id = detail
        .items
        .iter()
        .find(|i| i.text.as_deref() == Some("Paper towels"))
        .expect("towels present")
        .id;
    delete_shopping_list_item(towel_id)
        .await
        .expect("delete item");
    let detail = get_shopping_list(list_id).await.expect("get list 4");
    assert_eq!(detail.items.len(), 2);

    // Cleanup.
    delete_shopping_list(list_id).await.expect("delete list");
    delete_meal(meal_slug).await.expect("delete meal");
    delete_recipe(recipe_slug).await.expect("delete recipe");
}

#[tokio::test]
async fn empty_shopping_list_create_and_delete() {
    TestBook::new().await;
    let name = unique("Empty List");
    let id = create_shopping_list(name.clone()).await.expect("create");

    let listed = list_shopping_lists().await.expect("list");
    assert!(listed.iter().any(|l| l.id == id && l.name == name));

    let detail = get_shopping_list(id).await.expect("get");
    assert!(detail.items.is_empty());

    delete_shopping_list(id).await.expect("delete");
    let after = list_shopping_lists().await.expect("list after");
    assert!(!after.iter().any(|l| l.id == id));
}
