#[cfg(feature = "server")]
use {
    db::rpc::{Apply, ApplyOp, ListSince},
    server::{RequestContext, ingredient, meal, recipe, shopping_list},
};
use {
    db::{
        Name, Slug, Timestamp,
        id::{ShoppingListId, ShoppingListItemId},
        models::{
            book::Book,
            ingredient::{Ingredient, IngredientUpdate},
            meal::{Meal, MealBuilder, MealDetail},
            recipe::{Recipe, RecipeBuilder, RecipeDetail},
            shopping_list::{ShoppingList, ShoppingListDetail},
            shopping_list_item::ShoppingListItemInput,
            user::Current,
        },
        rpc::{ListResponse, Operation, OperationResponse},
    },
    dioxus::prelude::*,
};

pub mod auth;

#[get("/api/me", ctx: RequestContext)]
pub async fn me() -> Result<Current, ServerFnError> {
    Ok(ctx.current)
}

#[get("/api/books", mut ctx: RequestContext)]
pub async fn list_books() -> Result<Vec<Book>, ServerFnError> {
    let Some(user_id) = ctx.current.user.as_ref().map(|u| u.id) else {
        return Ok(Vec::new());
    };

    let books = server::book::list(ctx.conn(), user_id).await?;

    Ok(books)
}

/// Create a cookbook owned by the logged-in user (who becomes its admin).
#[post("/api/books/create", mut ctx: RequestContext)]
pub async fn create_book(name: Name, slug: Slug) -> Result<Book, ServerFnError> {
    let user_id = ctx.require_user()?.id;

    let book = server::book::create(ctx.conn(), user_id, name, slug).await?;

    Ok(book)
}

/// Log out and return the apex host to redirect the now-bookless user to.
#[post("/api/auth/logout")]
pub async fn logout() -> Result<(), ServerFnError> {
    RequestContext::logout().await?;
    Ok(())
}

#[get("/api/recipes", mut ctx: RequestContext)]
pub async fn list_recipes() -> Result<Vec<Recipe>, ServerFnError> {
    ctx.require_book()?;
    let list = recipe::list_all(&mut ctx).await?;
    Ok(list)
}

#[get("/api/recipes/:slug", mut ctx: RequestContext)]
pub async fn get_recipe(slug: String) -> Result<RecipeDetail, ServerFnError> {
    ctx.require_book()?;
    let recipe = recipe::get(&mut ctx, &slug).await?;
    Ok(recipe)
}

#[get("/api/ingredients", mut ctx: RequestContext)]
pub async fn list_ingredients() -> Result<Vec<Ingredient>, ServerFnError> {
    ctx.require_book()?;
    let ingredients = ingredient::list_all(&mut ctx).await?;
    Ok(ingredients)
}

/// Apply a partial edit to one ingredient. `input.id` selects the row; unset
/// fields are left unchanged (see [`IngredientUpdate`]).
#[post("/api/ingredients/update", mut ctx: RequestContext)]
pub async fn update_ingredient(input: IngredientUpdate) -> Result<Ingredient, ServerFnError> {
    ctx.require_admin()?;
    let ingredient = input.apply(&mut ctx).await.map_err(server::Error::from)?;

    Ok(ingredient)
}

/// Apply a batch of create/update/delete operations across models in order,
/// returning each resulting row. The client uses this to flush local edits.
#[post("/api/apply", mut ctx: RequestContext)]
pub async fn apply_ops(ops: Vec<Operation>) -> Result<Vec<OperationResponse>, ServerFnError> {
    ctx.require_admin()?;

    let mut responses = Vec::with_capacity(ops.len());
    // TODO: Do this in a transaction.
    for op in ops {
        responses.push(op.apply_op(&mut ctx).await.map_err(server::Error::from)?);
    }

    Ok(responses)
}

#[get("/api/ingredients/since", mut ctx: RequestContext)]
pub async fn list_ingredients_since(
    since: Timestamp,
) -> Result<ListResponse<Ingredient>, ServerFnError> {
    ctx.require_book()?;
    let page = Ingredient::list_since(&mut ctx, since)
        .await
        .map_err(server::Error::from)?;

    Ok(page)
}

/// Create or update a recipe and all of its steps/ingredients in one shot.
/// `RecipeBuilder::id` decides insert vs. update. Returns the canonical
/// `RecipeDetail` (server-generated slug, resolved ids) as a `get` would.
#[post("/api/recipes/upsert", mut ctx: RequestContext)]
pub async fn upsert_recipe(input: RecipeBuilder) -> Result<RecipeDetail, ServerFnError> {
    ctx.require_admin()?;
    let detail = recipe::upsert(input, &mut ctx).await?;
    Ok(detail)
}

#[post("/api/recipes/:slug/delete", mut ctx: RequestContext)]
pub async fn delete_recipe(slug: String) -> Result<(), ServerFnError> {
    ctx.require_admin()?;
    recipe::delete(&mut ctx, &slug).await?;
    Ok(())
}

#[get("/api/meals", mut ctx: RequestContext)]
pub async fn list_meals() -> Result<Vec<Meal>, ServerFnError> {
    ctx.require_book()?;
    let list = meal::list_all(&mut ctx).await?;
    Ok(list)
}

#[get("/api/meals/:slug", mut ctx: RequestContext)]
pub async fn get_meal(slug: String) -> Result<MealDetail, ServerFnError> {
    ctx.require_book()?;
    let meal = meal::get(&mut ctx, &slug).await?;
    Ok(meal)
}

/// Create or update a meal and its recipe rows in one shot. `MealBuilder::id`
/// decides insert vs. update. Returns the canonical `MealDetail`.
#[post("/api/meals/upsert", mut ctx: RequestContext)]
pub async fn upsert_meal(input: MealBuilder) -> Result<MealDetail, ServerFnError> {
    ctx.require_book()?;
    let detail = meal::upsert(input, &mut ctx).await?;
    Ok(detail)
}

#[post("/api/meals/:slug/delete", mut ctx: RequestContext)]
pub async fn delete_meal(slug: String) -> Result<(), ServerFnError> {
    ctx.require_book()?;
    meal::delete(&mut ctx, &slug).await?;
    Ok(())
}

#[get("/api/shopping-lists", mut ctx: RequestContext)]
pub async fn list_shopping_lists() -> Result<Vec<ShoppingList>, ServerFnError> {
    ctx.require_book()?;
    let list = shopping_list::list_all(&mut ctx).await?;
    Ok(list)
}

#[get("/api/shopping-lists/:id", mut ctx: RequestContext)]
pub async fn get_shopping_list(id: ShoppingListId) -> Result<ShoppingListDetail, ServerFnError> {
    ctx.require_book()?;
    let detail = shopping_list::get(&mut ctx, id).await?;
    Ok(detail)
}

/// Create an empty, named shopping list.
#[post("/api/shopping-lists/create", mut ctx: RequestContext)]
pub async fn create_shopping_list(name: String) -> Result<ShoppingListId, ServerFnError> {
    ctx.require_book()?;
    let id = shopping_list::create(&mut ctx, &name).await?;
    Ok(id)
}

/// Create a shopping list from a meal by aggregating its ingredients.
#[post("/api/shopping-lists/from-meal", mut ctx: RequestContext)]
pub async fn create_shopping_list_from_meal(
    meal_slug: String,
) -> Result<ShoppingListId, ServerFnError> {
    ctx.require_book()?;
    let id = shopping_list::create_from_meal(&mut ctx, &meal_slug).await?;
    Ok(id)
}

#[post("/api/shopping-lists/:id/delete", mut ctx: RequestContext)]
pub async fn delete_shopping_list(id: ShoppingListId) -> Result<(), ServerFnError> {
    ctx.require_book()?;
    shopping_list::delete(&mut ctx, id).await?;
    Ok(())
}

#[post("/api/shopping-lists/:list_id/items", mut ctx: RequestContext)]
pub async fn add_shopping_list_item(
    list_id: ShoppingListId,
    input: ShoppingListItemInput,
) -> Result<ShoppingListItemId, ServerFnError> {
    ctx.require_book()?;
    let id = shopping_list::add_item(&mut ctx, list_id, input).await?;
    Ok(id)
}

#[post("/api/shopping-list-items/:item_id/checked", mut ctx: RequestContext)]
pub async fn set_shopping_list_item_checked(
    item_id: ShoppingListItemId,
    checked: bool,
) -> Result<(), ServerFnError> {
    ctx.require_book()?;
    shopping_list::set_item_checked(&mut ctx, item_id, checked).await?;
    Ok(())
}

#[post("/api/shopping-list-items/:item_id/delete", mut ctx: RequestContext)]
pub async fn delete_shopping_list_item(item_id: ShoppingListItemId) -> Result<(), ServerFnError> {
    ctx.require_book()?;
    shopping_list::delete_item(&mut ctx, item_id).await?;
    Ok(())
}
