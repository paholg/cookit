#[cfg(feature = "development")]
use db::id::{BookId, UserId};
// The /api/dev/* handlers run their DB ops directly (no session), so they need
// the connection pool and the test-data helpers — both server-only.
#[cfg(all(feature = "server", feature = "development"))]
use server::{conn::get_conn, dev};
#[cfg(feature = "server")]
use {
    db::rpc::{Apply, ApplyOp, ListSince},
    server::{auth, ingredient, meal, recipe, session::Session, shopping_list},
};
use {
    db::{
        Timestamp,
        id::{ShoppingListId, ShoppingListItemId, UserRoleId},
        models::{
            ingredient::{Ingredient, IngredientUpdate},
            meal::{Meal, MealBuilder, MealDetail},
            recipe::{Recipe, RecipeBuilder, RecipeDetail},
            shopping_list::{ShoppingList, ShoppingListDetail},
            shopping_list_item::ShoppingListItemInput,
            user::CurrentUser,
        },
        rpc::{ListResponse, Operation, OperationResponse},
    },
    dioxus::prelude::*,
};

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
    auth::clear_session_cookie();
    Ok(())
}

#[get("/api/recipes")]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = recipe::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/recipes/:slug")]
pub async fn get_recipe(slug: String) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let recipe = recipe::get(&mut session, &slug).await?;
    Ok(recipe)
}

#[get("/api/ingredients")]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    let mut session = Session::require().await?;
    let ingredients = ingredient::list_all(&mut session).await?;
    Ok(ingredients)
}

/// Apply a partial edit to one ingredient. `input.id` selects the row; unset
/// fields are left unchanged (see [`IngredientUpdate`]).
#[post("/api/ingredients/update")]
pub async fn update_ingredient(input: IngredientUpdate) -> Result<Ingredient, ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    let ingredient = input
        .apply(&mut session)
        .await
        .map_err(server::Error::from)?;

    Ok(ingredient)
}

/// Apply a batch of create/update/delete operations across models in order,
/// returning each resulting row. The client uses this to flush local edits.
#[post("/api/apply")]
pub async fn apply_ops(ops: Vec<Operation>) -> Result<Vec<OperationResponse>, ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;

    let mut responses = Vec::with_capacity(ops.len());
    // TODO: Do this in a transaction.
    for op in ops {
        responses.push(
            op.apply_op(&mut session)
                .await
                .map_err(server::Error::from)?,
        );
    }

    Ok(responses)
}

#[get("/api/ingredients/since")]
pub async fn list_ingredients_since(
    since: Timestamp,
) -> Result<ListResponse<Ingredient>, ServerFnError> {
    let mut session = Session::require().await?;
    let page = Ingredient::list_since(&mut session, since)
        .await
        .map_err(server::Error::from)?;

    Ok(page)
}

/// Create or update a recipe and all of its steps/ingredients in one shot.
/// `RecipeBuilder::id` decides insert vs. update. Returns the canonical
/// `RecipeDetail` (server-generated slug, resolved ids) as a `get` would.
#[post("/api/recipes/upsert")]
pub async fn upsert_recipe(input: RecipeBuilder) -> Result<RecipeDetail, ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    let detail = recipe::upsert(input, &mut session).await?;
    Ok(detail)
}

#[post("/api/recipes/:slug/delete")]
pub async fn delete_recipe(slug: String) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    session.require_admin()?;
    recipe::delete(&mut session, &slug).await?;
    Ok(())
}

#[get("/api/meals")]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = meal::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/meals/:slug")]
pub async fn get_meal(slug: String) -> Result<MealDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let meal = meal::get(&mut session, &slug).await?;
    Ok(meal)
}

/// Create or update a meal and its recipe rows in one shot. `MealBuilder::id`
/// decides insert vs. update. Returns the canonical `MealDetail`.
#[post("/api/meals/upsert")]
pub async fn upsert_meal(input: MealBuilder) -> Result<MealDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let detail = meal::upsert(input, &mut session).await?;
    Ok(detail)
}

#[post("/api/meals/:slug/delete")]
pub async fn delete_meal(slug: String) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    meal::delete(&mut session, &slug).await?;
    Ok(())
}

#[get("/api/shopping-lists")]
pub async fn list_shopping_lists() -> Result<Vec<ShoppingList>, ServerFnError> {
    let mut session = Session::require().await?;
    let list = shopping_list::list_all(&mut session).await?;
    Ok(list)
}

#[get("/api/shopping-lists/:id")]
pub async fn get_shopping_list(id: ShoppingListId) -> Result<ShoppingListDetail, ServerFnError> {
    let mut session = Session::require().await?;
    let detail = shopping_list::get(&mut session, id).await?;
    Ok(detail)
}

/// Create an empty, named shopping list.
#[post("/api/shopping-lists/create")]
pub async fn create_shopping_list(name: String) -> Result<ShoppingListId, ServerFnError> {
    let mut session = Session::require().await?;
    let id = shopping_list::create(&mut session, &name).await?;
    Ok(id)
}

/// Create a shopping list from a meal by aggregating its ingredients.
#[post("/api/shopping-lists/from-meal")]
pub async fn create_shopping_list_from_meal(
    meal_slug: String,
) -> Result<ShoppingListId, ServerFnError> {
    let mut session = Session::require().await?;
    let id = shopping_list::create_from_meal(&mut session, &meal_slug).await?;
    Ok(id)
}

#[post("/api/shopping-lists/:id/delete")]
pub async fn delete_shopping_list(id: ShoppingListId) -> Result<(), ServerFnError> {
    let mut session = Session::require().await?;
    shopping_list::delete(&mut session, id).await?;
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
