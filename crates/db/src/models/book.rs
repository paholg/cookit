use {
    crate::id::{BookId, UserId},
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{models::user::User, schema::books},
    diesel::prelude::*,
};

/// The unit of tenancy for CookIt.
///
/// Virtually every table has a book_id column, placing it in a user's "cookbook".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(User, foreign_key = owner_id)))]
pub struct Book {
    pub id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub name: String,
    pub slug: String,
    pub owner_id: UserId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = books))]
pub struct BookNew<'a> {
    pub name: &'a str,
    pub slug: &'a str,
    pub owner_id: UserId,
}
