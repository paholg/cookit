use dioxus::prelude::*;
use types::{
    CurrentUser, Ingredient, IngredientUpdate, Meal, MealDetail, NewMeal, NewRecipe, Recipe,
    RecipeDetail,
};

/// Build the axum router for `/auth/*` endpoints. Merge this into the dioxus
/// router from the server entrypoint.
#[cfg(feature = "server")]
pub async fn auth_router() -> dioxus::server::axum::Router {
    server::auth::router().await
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

#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    server::auth::require_user().await?;
    server::ops::list_meals()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/meals/:id")]
pub async fn get_meal(id: i64) -> Result<MealDetail, ServerFnError> {
    server::auth::require_user().await?;
    server::ops::get_meal(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("meal {id} not found")))
}

#[post("/api/meals")]
pub async fn create_meal(input: NewMeal) -> Result<i64, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::create_meal(input, actor.id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/meals/:id/update")]
pub async fn update_meal(id: i64, input: NewMeal) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::update_meal(id, input, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/meals/:id/delete")]
pub async fn delete_meal(id: i64) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_meal(id, actor.id, actor.is_admin)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
