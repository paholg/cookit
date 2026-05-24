use dioxus::prelude::*;
use types::{
    Ingredient, IngredientUpdate, Meal, MealDetail, NewMeal, NewRecipe, Recipe, RecipeDetail,
};

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
    server::ops::list_ingredients()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/ingredients/:id/update")]
pub async fn update_ingredient(id: i64, input: IngredientUpdate) -> Result<(), ServerFnError> {
    server::ops::update_ingredient(id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes")]
pub async fn create_recipe(input: NewRecipe) -> Result<i64, ServerFnError> {
    server::ops::create_recipe(input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes/:id/update")]
pub async fn update_recipe(id: i64, input: NewRecipe) -> Result<(), ServerFnError> {
    server::ops::update_recipe(id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    server::ops::list_meals()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[get("/api/meals/:id")]
pub async fn get_meal(id: i64) -> Result<MealDetail, ServerFnError> {
    server::ops::get_meal(id)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new(format!("meal {id} not found")))
}

#[post("/api/meals")]
pub async fn create_meal(input: NewMeal) -> Result<i64, ServerFnError> {
    server::ops::create_meal(input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/meals/:id/update")]
pub async fn update_meal(id: i64, input: NewMeal) -> Result<(), ServerFnError> {
    server::ops::update_meal(id, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
