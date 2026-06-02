use {
    crate::{
        db::models::{
            ingredient::{Ingredient, IngredientUpdate},
            meal::{Meal, MealBuilder, MealDetail},
            recipe::{Recipe, RecipeBuilder, RecipeDetail},
            shopping_list::{ShoppingList, ShoppingListDetail},
            shopping_list_item::ShoppingListItemInput,
        },
        id::{IngredientId, ShoppingListId, ShoppingListItemId, UserRoleId},
        session::CurrentUser,
    },
    dioxus::prelude::*,
};

#[cfg(feature = "development")]
use crate::id::{BookId, UserId};

// Server-only: handler bodies use the DB session and the shopping-list ops,
// which don't exist on the wasm client.
#[cfg(feature = "server")]
use crate::{db::models::shopping_list, session::Session};

// The /api/dev/* handlers run their DB ops directly (no session), so they need
// the connection pool and the test-data helpers — both server-only.
#[cfg(all(feature = "server", feature = "development"))]
use crate::{db::conn::get_conn, dev};

#[get("/api/me")]
pub async fn me() -> Result<Option<CurrentUser>, ServerFnError> {
    let session = Session::from_request().await?;
    Ok(session.map(|s| s.current_user()))
}

/// Log in as a specific `user_role` (used by the e2e tests). Issues a session
/// cookie and returns the now-current user.
#[post("/api/auth/login")]
pub async fn login(user_role_id: UserRoleId) -> Result<CurrentUser, ServerFnError> {
    let session = Session::login(user_role_id).await?;
    Ok(session.current_user())
}

/// Log in as the first user/book. Powers the dev "Log in" button until real
/// login is restored.
#[post("/api/auth/login-first")]
pub async fn login_as_first() -> Result<CurrentUser, ServerFnError> {
    let session = Session::login_first().await?;
    Ok(session.current_user())
}

#[post("/api/auth/logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    crate::auth::clear_session_cookie();
    Ok(())
}

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = Recipe::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/recipes/:slug")]
pub async fn get_recipe(slug: String) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let recipe = RecipeDetail::get(&mut session, &slug).await?;
    Ok(recipe)
}

#[get("/api/ingredients")]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    let mut session = Session::require().await?;
    let ingredients = Ingredient::list_all(&mut session).await?;
    Ok(ingredients)
}

#[post("/api/ingredients/:id/update")]
pub async fn update_ingredient(
    id: IngredientId,
    input: IngredientUpdate,
) -> Result<Ingredient, ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    let ingredient = input.apply(id, &mut session).await?;

    Ok(ingredient)
}

/// Create or update a recipe and all of its steps/ingredients in one shot.
/// `RecipeBuilder::id` decides insert vs. update. Returns the canonical
/// `RecipeDetail` (server-generated slug, resolved ids) as a `get` would.
#[post("/api/recipes/upsert")]
pub async fn upsert_recipe(input: RecipeBuilder) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    let detail = input.upsert(&mut session).await?;
    Ok(detail)
}

#[post("/api/recipes/:slug/delete")]
pub async fn delete_recipe(slug: String) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    Recipe::delete(&mut session, &slug).await?;
    Ok(())
}

#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = Meal::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/meals/:slug")]
pub async fn get_meal(slug: String) -> Result<MealDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let meal = MealDetail::get(&mut session, &slug).await?;
    Ok(meal)
}

/// Create or update a meal and its recipe rows in one shot. `MealBuilder::id`
/// decides insert vs. update. Returns the canonical `MealDetail`.
#[post("/api/meals/upsert")]
pub async fn upsert_meal(input: MealBuilder) -> Result<MealDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let detail = input.upsert(&mut session).await?;
    Ok(detail)
}

#[post("/api/meals/:slug/delete")]
pub async fn delete_meal(slug: String) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    Meal::delete(&mut session, &slug).await?;
    Ok(())
}

#[get("/api/shopping-lists")]
pub async fn list_shopping_lists() -> Result<Vec<ShoppingList>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = ShoppingList::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/shopping-lists/:id")]
pub async fn get_shopping_list(id: ShoppingListId) -> Result<ShoppingListDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let detail = ShoppingListDetail::get(&mut session, id).await?;
    Ok(detail)
}

/// Create an empty, named shopping list.
#[post("/api/shopping-lists/create")]
pub async fn create_shopping_list(name: String) -> Result<ShoppingListId, ServerFnError> {
    let mut session = Session::require().await?;
    let id = ShoppingList::create(&mut session, &name).await?;
    Ok(id)
}

/// Create a shopping list from a meal by aggregating its ingredients.
#[post("/api/shopping-lists/from-meal")]
pub async fn create_shopping_list_from_meal(
    meal_slug: String,
) -> Result<ShoppingListId, ServerFnError> {
    let mut session = Session::require().await?;
    let id = ShoppingList::create_from_meal(&mut session, &meal_slug).await?;
    Ok(id)
}

#[post("/api/shopping-lists/:id/delete")]
pub async fn delete_shopping_list(id: ShoppingListId) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    ShoppingList::delete(&mut session, id).await?;
    Ok(())
}

#[post("/api/shopping-lists/:list_id/items")]
pub async fn add_shopping_list_item(
    list_id: ShoppingListId,
    input: ShoppingListItemInput,
) -> Result<ShoppingListItemId, ServerFnError> {
    let mut session = Session::require().await?;
    let id = shopping_list::add_item(&mut session, list_id, input).await?;
    Ok(id)
}

#[post("/api/shopping-list-items/:item_id/checked")]
pub async fn set_shopping_list_item_checked(
    item_id: ShoppingListItemId,
    checked: bool,
) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    shopping_list::set_item_checked(&mut session, item_id, checked).await?;
    Ok(())
}

#[post("/api/shopping-list-items/:item_id/delete")]
pub async fn delete_shopping_list_item(item_id: ShoppingListItemId) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    shopping_list::delete_item(&mut session, item_id).await?;
    Ok(())
}

/// The ids of the throwaway admin user/book/role created by [`dev_setup`]. The
/// e2e suite logs in with `user_role_id` and hands `user_id`/`book_id` back to
/// [`dev_teardown`].
#[cfg(feature = "development")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DevTestData {
    pub user_id: UserId,
    pub book_id: BookId,
    pub user_role_id: UserRoleId,
}

/// Create an isolated admin user + book + role for an e2e run. Unauthenticated
/// on purpose — it mints the very user the suite logs in as. Only compiled with
/// the `development` feature, so it never ships in production.
#[cfg(feature = "development")]
#[post("/api/dev/setup")]
pub async fn dev_setup() -> Result<DevTestData, ServerFnError> {
    let mut conn = get_conn().await?;
    let (user_id, book_id, user_role_id) = dev::create_test_book(&mut conn).await?;

    Ok(DevTestData {
        user_id,
        book_id,
        user_role_id,
    })
}

/// Delete the user + book created by [`dev_setup`]; cascades clean up the role
/// and all book-scoped rows.
#[cfg(feature = "development")]
#[post("/api/dev/teardown")]
pub async fn dev_teardown(user_id: UserId, book_id: BookId) -> Result<(), ServerFnError> {
    let mut conn = get_conn().await?;
    dev::delete_test_book(&mut conn, user_id, book_id).await?;

    Ok(())
}
