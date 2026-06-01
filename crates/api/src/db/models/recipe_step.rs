use {
    crate::id::{BookId, RecipeId, RecipeStepId},
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
pub(crate) struct RecipeStep {
    pub(crate) id: RecipeStepId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) recipe_id: RecipeId,
    pub(crate) position: i32,
    pub(crate) text: String,
    pub(crate) duration_s: Option<i32>,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = recipe_steps))]
pub(crate) struct NewRecipeStep<'a> {
    pub(crate) book_id: BookId,
    pub(crate) recipe_id: RecipeId,
    pub(crate) position: i32,
    pub(crate) text: &'a str,
    pub(crate) duration_s: Option<i32>,
}
