use dioxus::prelude::*;
#[cfg(feature = "dev-auth")]
use types::DevUser;
use types::{CurrentUser, Ingredient, IngredientUpdate, NewRecipe, Recipe, RecipeDetail};

pub mod meals;
mod remote;

/// Build the axum router for `/auth/*` endpoints. Merge this into the dioxus
/// router from the server entrypoint.
#[cfg(feature = "server")]
pub async fn auth_router() -> dioxus::server::axum::Router {
    server::auth::router().await
}

/// Middleware that logs any 5xx response so server failures aren't silent.
#[cfg(feature = "server")]
pub use server::middleware::log_server_errors;

/// Dev-only roster of users used by the in-navbar "log in as" `<select>`.
/// The server function only exists in builds compiled with `dev-auth`.
#[cfg(feature = "dev-auth")]
#[get("/api/dev-users")]
pub async fn list_dev_users() -> Result<Vec<DevUser>, ServerFnError> {
    server::auth::list_dev_users()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/me")]
pub async fn me() -> Result<Option<CurrentUser>, ServerFnError> {
    Ok(server::auth::current_user().await.map(|u| CurrentUser {
        id: u.id,
        name: u.name,
        email: u.email,
        is_admin: u.is_admin,
    }))
}

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    server::ops::list_recipes()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/recipes/:id")]
pub async fn get_recipe(id: i64) -> Result<RecipeDetail, ServerFnError> {
    server::ops::get_recipe(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("recipe {id} not found")))
}

#[get("/api/ingredients")]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    server::auth::require_user().await?;
    server::ops::list_ingredients()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/ingredients/:id/update")]
pub async fn update_ingredient(id: i64, input: IngredientUpdate) -> Result<(), ServerFnError> {
    server::auth::require_admin().await?;
    server::ops::update_ingredient(id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes")]
pub async fn create_recipe(input: NewRecipe) -> Result<i64, ServerFnError> {
    server::auth::require_admin().await?;
    server::ops::create_recipe(input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes/:id/update")]
pub async fn update_recipe(id: i64, input: NewRecipe) -> Result<(), ServerFnError> {
    server::auth::require_admin().await?;
    server::ops::update_recipe(id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
