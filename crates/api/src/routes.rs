use {
    crate::{
        db::models::{
            ingredient::{Ingredient, IngredientUpdate},
            recipe::{Recipe, RecipeBuilder, RecipeDetail},
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

/// Create or update a recipe and all of its steps/ingredients in one shot.
/// `RecipeBuilder::id` decides insert vs. update. Returns the canonical
/// `RecipeDetail` (server-generated slug, resolved ids) as a `get` would.
#[post("/api/recipes/upsert")]
pub async fn upsert_recipe(input: RecipeBuilder) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::create().await?;
    session.require_admin()?;
    let detail = input.upsert(&mut session).await?;
    Ok(detail)
}

#[post("/api/recipes/:slug/delete")]
pub async fn delete_recipe(slug: String) -> Result<(), ServerFnError> {
    let mut session = Session::create().await?;
    session.require_admin()?;
    Recipe::delete(&mut session, &slug).await?;
    Ok(())
}
