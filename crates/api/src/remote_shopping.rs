//! Server-backed shopping list endpoints.

use {
    dioxus::prelude::*,
    std::collections::HashMap,
    types::{
        GrocerySection, NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail,
        id::{IngredientId, ShoppingListId, ShoppingListItemId},
    },
};

#[get("/api/shopping-lists")]
pub async fn list_shopping_lists() -> Result<Vec<ShoppingList>, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::list_shopping_lists(actor.book_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/shopping-lists/:id")]
pub async fn get_shopping_list(id: ShoppingListId) -> Result<ShoppingListDetail, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::get_shopping_list(actor.book_id, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("shopping list not found".to_string()))
}

#[post("/api/shopping-lists")]
pub async fn create_shopping_list(input: NewShoppingList) -> Result<ShoppingListId, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::create_shopping_list(actor.book_id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-lists/:id/delete")]
pub async fn delete_shopping_list(id: ShoppingListId) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_shopping_list(actor.book_id, id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-lists/:list_id/items")]
pub async fn add_shopping_list_item(
    list_id: ShoppingListId,
    item: NewShoppingListItem,
) -> Result<ShoppingListItemId, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::add_shopping_list_item(actor.book_id, list_id, item)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-list-items/:item_id/checked")]
pub async fn set_shopping_list_item_checked(
    item_id: ShoppingListItemId,
    checked: bool,
) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::set_shopping_list_item_checked(actor.book_id, item_id, checked)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-list-items/:item_id/delete")]
pub async fn delete_shopping_list_item(item_id: ShoppingListItemId) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_shopping_list_item(actor.book_id, item_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/ingredient-sections")]
pub async fn list_ingredient_sections()
-> Result<HashMap<IngredientId, Option<GrocerySection>>, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::list_ingredient_sections(actor.book_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
