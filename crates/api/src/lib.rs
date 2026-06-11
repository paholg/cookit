pub mod routes;

pub use {
    db::{
        duration, grocery_section, helpers, id,
        models::{
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
            user::CurrentUser,
        },
        unit,
    },
    routes::*,
};

#[cfg(feature = "development")]
pub const APP_NAME: &str = "[dev] CookIt!";

#[cfg(not(feature = "development"))]
pub const APP_NAME: &str = "CookIt!";
