pub mod db;
pub mod duration;
pub mod error;
pub mod grocery_section;
pub mod helpers;
pub mod id;
pub mod routes;
pub mod session;
pub mod unit;

// Server-only modules: pull in axum/figment/cookie, which don't build for wasm.
#[cfg(feature = "server")]
pub mod auth;
#[cfg(feature = "server")]
pub mod config;
#[cfg(feature = "server")]
pub mod dev;
#[cfg(feature = "server")]
pub mod middleware;

#[cfg(feature = "server")]
pub use middleware::log_server_errors;
pub use {
    db::models::{
        ingredient::{Ingredient, IngredientUpdate},
        meal::{Meal, MealBuilder, MealDetail, MealError},
        meal_recipe::{MealRecipeBuilder, MealRecipeDetail, MealRecipeError},
        recipe::{Recipe, RecipeBuilder, RecipeDetail, RecipeError},
        recipe_step::{RecipeStepBuilder, RecipeStepDetail, RecipeStepError},
        recipe_step_ingredient::{
            RecipeStepIngredientBuilder, RecipeStepIngredientDetail, RecipeStepIngredientError,
        },
        shopping_list::{ShoppingList, ShoppingListDetail},
        shopping_list_item::{ShoppingListItem, ShoppingListItemInput, ShoppingListItemView},
    },
    error::Result,
    routes::*,
};
