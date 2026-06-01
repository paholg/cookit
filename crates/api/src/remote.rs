//! Server-backed meal endpoints.

use {
    dioxus::prelude::*,
    types::{Meal, MealDetail, NewMeal},
};

#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::list_meals(actor.book_id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/meals/:slug")]
pub async fn get_meal(slug: String) -> Result<MealDetail, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::get_meal_by_slug(actor.book_id, &slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("meal `{slug}` not found")))
}

#[post("/api/meals")]
pub async fn create_meal(input: NewMeal) -> Result<String, ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::create_meal(actor.book_id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/meals/:slug/update")]
pub async fn update_meal(slug: String, input: NewMeal) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::update_meal(actor.book_id, &slug, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/meals/:slug/delete")]
pub async fn delete_meal(slug: String) -> Result<(), ServerFnError> {
    let actor = server::auth::require_user().await?;
    server::ops::delete_meal(actor.book_id, &slug)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
