use {
    crate::id::{BookId, MealId},
    jiff::Timestamp,
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{models::book::Book, prelude::*, schema::meals};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
pub(crate) struct Meal {
    pub(crate) id: MealId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: Timestamp,
    pub(crate) slug: String,
    pub(crate) name: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = meals))]
pub(crate) struct NewMeal<'a> {
    pub(crate) book_id: BookId,
    pub(crate) slug: &'a str,
    pub(crate) name: &'a str,
}
