#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, ingredient::Ingredient, recipe_step::RecipeStep},
    prelude::*,
    schema::recipe_step_ingredients,
};
use crate::id::{BookId, IngredientId, RecipeStepId, RecipeStepIngredientId};

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(RecipeStep, foreign_key = step_id)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Ingredient)))]
pub(crate) struct RecipeStepIngredient {
    pub(crate) id: RecipeStepIngredientId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) step_id: RecipeStepId,
    pub(crate) position: i32,
    pub(crate) quantity: Option<f64>,
    pub(crate) unit_kind: Option<String>,
    pub(crate) unit: Option<String>,
    pub(crate) ingredient_id: IngredientId,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = recipe_step_ingredients))]
pub(crate) struct NewRecipeStepIngredient<'a> {
    pub(crate) book_id: BookId,
    pub(crate) step_id: RecipeStepId,
    pub(crate) position: i32,
    pub(crate) quantity: Option<f64>,
    pub(crate) unit_kind: Option<&'a str>,
    pub(crate) unit: Option<&'a str>,
    pub(crate) ingredient_id: IngredientId,
}
