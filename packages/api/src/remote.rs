//! Server-backed meal endpoints. These are the implementation detail behind
//! the meal calls in [`crate::meals`] for the authenticated/server-id branch.
//! Routes and behaviour are unchanged from when they lived at the crate root —
//! the existing `/api/meals/*` endpoints continue to serve traffic.

use dioxus::prelude::*;
use types::{Meal, MealDetail, NewMeal};

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
