use {
    crate::{
        db::models::recipe_step_ingredient::RecipeStepIngredientDetail,
        id::{BookId, RecipeId, RecipeStepId},
    },
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, recipe::Recipe},
    prelude::*,
    schema::recipe_steps,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Recipe)))]
pub struct RecipeStep {
    pub id: RecipeStepId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub recipe_id: RecipeId,
    pub position: i32,
    pub text: String,
    pub duration_s: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStepDetail {
    pub step: RecipeStep,
    pub ingredients: Vec<RecipeStepIngredientDetail>,
}
