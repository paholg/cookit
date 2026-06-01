use {
    crate::id::{BookId, MealId, MealRecipeId, RecipeId},
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, meal::Meal, recipe::Recipe},
    prelude::*,
    schema::meal_recipes,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Meal)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Recipe)))]
pub(crate) struct MealRecipe {
    pub(crate) id: MealRecipeId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) meal_id: MealId,
    pub(crate) recipe_id: RecipeId,
    pub(crate) multiplier: f64,
    pub(crate) position: i32,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = meal_recipes))]
pub(crate) struct NewMealRecipe {
    pub(crate) book_id: BookId,
    pub(crate) meal_id: MealId,
    pub(crate) recipe_id: RecipeId,
    pub(crate) multiplier: f64,
    pub(crate) position: i32,
}
