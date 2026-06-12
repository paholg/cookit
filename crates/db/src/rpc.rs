//! Shared plumbing for the generated CRUD/sync layer.
//!
//! The wire types here (`ListResponse`, `Operation`, `OperationResponse`) are
//! always compiled so the wasm client can build requests and read responses.
//! The traits that actually touch the database (`RpcContext`, `Apply`,
//! `ListSince`, `ApplyOp`) and the `enum_dispatch` glue are gated behind the
//! `server` feature.

#[cfg(feature = "server")]
use {
    crate::id::BookId,
    diesel_async::AsyncPgConnection,
    enum_dispatch::enum_dispatch,
    std::{future::Future, pin::Pin},
};
use {
    crate::{
        Timestamp,
        models::{
            ingredient::{
                IngredientCreate, IngredientDelete, IngredientResponse, IngredientUpdate,
            },
            meal::{MealCreate, MealDelete, MealResponse, MealUpdate},
            meal_recipe::{
                MealRecipeCreate, MealRecipeDelete, MealRecipeResponse, MealRecipeUpdate,
            },
            recipe::{RecipeCreate, RecipeDelete, RecipeResponse, RecipeUpdate},
            recipe_step::{
                RecipeStepCreate, RecipeStepDelete, RecipeStepResponse, RecipeStepUpdate,
            },
            recipe_step_ingredient::{
                RecipeStepIngredientCreate, RecipeStepIngredientDelete,
                RecipeStepIngredientResponse, RecipeStepIngredientUpdate,
            },
            shopping_list::{
                ShoppingListCreate, ShoppingListDelete, ShoppingListResponse, ShoppingListUpdate,
            },
            shopping_list_item::{
                ShoppingListItemCreate, ShoppingListItemDelete, ShoppingListItemResponse,
                ShoppingListItemUpdate,
            },
        },
    },
    serde::{Deserialize, Serialize},
};

/// Max rows returned by a single `list_since` page.
pub const PAGE_SIZE: i64 = 100;

/// One page of sync results. `cursor` is `Some(last.updated_at)` when the page
/// was full (more may remain), else `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub records: Vec<T>,
    pub cursor: Option<Timestamp>,
}

/// A single create/update/delete the client wants applied. The wasm client
/// builds these with the explicit variant constructors; the `enum_dispatch`
/// `From`/`apply_op` glue is server-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", enum_dispatch(ApplyOp))]
pub enum Operation {
    IngredientCreate(IngredientCreate),
    IngredientUpdate(IngredientUpdate),
    IngredientDelete(IngredientDelete),

    MealCreate(MealCreate),
    MealUpdate(MealUpdate),
    MealDelete(MealDelete),

    MealRecipeCreate(MealRecipeCreate),
    MealRecipeUpdate(MealRecipeUpdate),
    MealRecipeDelete(MealRecipeDelete),

    RecipeCreate(RecipeCreate),
    RecipeUpdate(RecipeUpdate),
    RecipeDelete(RecipeDelete),

    RecipeStepCreate(RecipeStepCreate),
    RecipeStepUpdate(RecipeStepUpdate),
    RecipeStepDelete(RecipeStepDelete),

    RecipeStepIngredientCreate(RecipeStepIngredientCreate),
    RecipeStepIngredientUpdate(RecipeStepIngredientUpdate),
    RecipeStepIngredientDelete(RecipeStepIngredientDelete),

    ShoppingListCreate(ShoppingListCreate),
    ShoppingListUpdate(ShoppingListUpdate),
    ShoppingListDelete(ShoppingListDelete),

    ShoppingListItemCreate(ShoppingListItemCreate),
    ShoppingListItemUpdate(ShoppingListItemUpdate),
    ShoppingListItemDelete(ShoppingListItemDelete),
}

/// The row an applied [`Operation`] produced, returned to the client.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OperationResponse {
    Ingredient(IngredientResponse),
    Meal(MealResponse),
    MealRecipe(MealRecipeResponse),
    Recipe(RecipeResponse),
    RecipeStep(RecipeStepResponse),
    RecipeStepIngredient(RecipeStepIngredientResponse),
    ShoppingList(ShoppingListResponse),
    ShoppingListItem(ShoppingListItemResponse),
}

/// The context a generated DB operation runs against: a connection plus the
/// book it is scoped to. Implemented by `server::Session`, so `db` never has to
/// depend on the `server` crate.
// `Send` supertrait so `dyn RpcContext: Send`, which the boxed `Send` futures in
// `ApplyOp` require. `Session` is `Send`, so this costs nothing.
#[cfg(feature = "server")]
pub trait RpcContext: Send {
    fn conn(&mut self) -> &mut AsyncPgConnection;
    fn book_id(&self) -> BookId;
}

/// A create/update/delete record that knows how to apply itself and return the
/// canonical row. Implemented (via `#[diesel_rpc]`) by each `…Create`,
/// `…Update`, and `…Delete` struct.
#[cfg(feature = "server")]
#[allow(async_fn_in_trait)]
pub trait Apply {
    type Response;

    async fn apply(self, ctx: &mut dyn RpcContext) -> crate::error::Result<Self::Response>;
}

/// A response type that can be paged by `updated_at` for sync. Implemented by
/// each `…Response` struct.
#[cfg(feature = "server")]
#[allow(async_fn_in_trait)]
pub trait ListSince: Sized {
    async fn list_since(
        ctx: &mut dyn RpcContext,
        since: Timestamp,
    ) -> crate::error::Result<ListResponse<Self>>;
}

/// Uniform-signature bridge used by `enum_dispatch` to dispatch [`Operation`].
///
/// `enum_dispatch` is static `match` dispatch, so every variant must share one
/// return type — hence the erased `OperationResponse` and the hand-boxed future
/// (a native `async fn` would give each variant a distinct opaque future that
/// the generated `match` arms couldn't unify).
#[cfg(feature = "server")]
#[enum_dispatch]
pub trait ApplyOp {
    fn apply_op(
        self,
        ctx: &mut dyn RpcContext,
    ) -> Pin<Box<dyn Future<Output = crate::error::Result<OperationResponse>> + Send + '_>>;
}
