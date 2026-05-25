//! Server-backed shopping list endpoints, mirroring [`crate::remote`] for
//! meals. Routes follow the same `/api/shopping-lists/*` shape used elsewhere.

use std::collections::HashMap;

use dioxus::prelude::*;
use types::{
    GrocerySection, NewShoppingList, NewShoppingListItem, ShoppingList, ShoppingListDetail,
};

#[get("/api/shopping-lists")]
pub async fn list_shopping_lists() -> Result<Vec<ShoppingList>, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::list_shopping_lists(actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/shopping-lists/:id")]
pub async fn get_shopping_list(id: i64) -> Result<ShoppingListDetail, ServerFnError> {
    server::auth::require_user().await?;
    server::ops::get_shopping_list(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("shopping list {id} not found")))
}

#[post("/api/shopping-lists")]
pub async fn create_shopping_list(input: NewShoppingList) -> Result<i64, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::create_shopping_list(input, actor.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-lists/:id/delete")]
pub async fn delete_shopping_list(id: i64) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_shopping_list(id, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-lists/:list_id/items")]
pub async fn add_shopping_list_item(
    list_id: i64,
    item: NewShoppingListItem,
) -> Result<i64, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::add_shopping_list_item(list_id, item, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-list-items/:item_id/checked")]
pub async fn set_shopping_list_item_checked(
    item_id: i64,
    checked: bool,
) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::set_shopping_list_item_checked(item_id, checked, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/shopping-list-items/:item_id/delete")]
pub async fn delete_shopping_list_item(item_id: i64) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_shopping_list_item(item_id, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Public lookup of grocery section by ingredient id, used during the
/// meal-to-shopping-list aggregation step. Public because recipes (and thus
/// the ingredient references inside them) are already public-readable —
/// withholding the section would just force the unauthenticated UI to lump
/// everything into "Other".
#[get("/api/ingredient-sections")]
pub async fn list_ingredient_sections()
-> Result<HashMap<i64, Option<GrocerySection>>, ServerFnError> {
    server::ops::list_ingredient_sections()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
