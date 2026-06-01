use {
    crate::{
        db::models::{
            ingredient::{Ingredient, IngredientUpdate},
            recipe::{Recipe, RecipeDetail},
        },
        id::IngredientId,
        session::{CurrentUser, Session},
    },
    dioxus::prelude::*,
};

#[get("/api/me")]
pub async fn me() -> Result<Option<CurrentUser>, ServerFnError> {
    let session = Session::create().await?;
    Ok(Some(session.current_user()))
}

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    let mut session = Session::create().await?;
    let list = Recipe::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/recipes/:slug")]
pub async fn get_recipe(slug: String) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::create().await?;
    let recipe = RecipeDetail::get(&mut session, &slug).await?;
    Ok(recipe)
}

#[get("/api/ingredients")]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    let mut session = Session::create().await?;
    let ingredients = Ingredient::list_all(&mut session).await?;
    Ok(ingredients)
}

#[post("/api/ingredients/:id/update")]
pub async fn update_ingredient(
    id: IngredientId,
    input: IngredientUpdate,
) -> Result<Ingredient, ServerFnError> {
    let mut session = Session::create().await?;
    session.require_admin()?;
    let ingredient = input.apply(id, &mut session).await?;

    Ok(ingredient)
}

#[post("/api/recipes")]
pub async fn create_recipe(input: NewRecipe) -> Result<String, ServerFnError> {
    let mut session = Session::create().await?;
    server::auth::require_admin().await?;
    server::ops::create_recipe(input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes/:key/update")]
pub async fn update_recipe(key: String, input: NewRecipe) -> Result<(), ServerFnError> {
    let mut session = Session::create().await?;
    server::auth::require_admin().await?;
    server::ops::update_recipe(&key, input)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[post("/api/recipes/:key/delete")]
pub async fn delete_recipe(key: String) -> Result<(), ServerFnError> {
    let mut session = Session::create().await?;
    server::auth::require_admin().await?;
    server::ops::delete_recipe(&key)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
