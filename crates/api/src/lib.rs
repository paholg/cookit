pub mod config;
pub mod db;
pub mod middleware;
// FIXME
// pub mod ops;
pub mod duration;
pub mod error;
pub mod grocery_section;
pub mod helpers;
pub mod id;
pub mod routes;
pub mod session;
pub mod unit;

pub use error::Result;

// Public surface for clients (the web crate). The diesel row structs stay
// crate-private; these are the shared read DTOs, edit builders, and validation
// errors that cross the wire and bind the forms.
pub use {
    db::models::{
        ingredient::Ingredient,
        recipe::{RecipeBuilder, RecipeDetail, RecipeError},
        recipe_step::{RecipeStepBuilder, RecipeStepDetail, RecipeStepError},
        recipe_step_ingredient::{
            RecipeStepIngredientBuilder, RecipeStepIngredientDetail, RecipeStepIngredientError,
        },
    },
    routes::*,
};
