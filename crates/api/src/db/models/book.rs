use {
    crate::id::{BookId, UserId},
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{models::user::User, prelude::*, schema::books};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(User, foreign_key = owner_id)))]
pub(crate) struct Book {
    pub(crate) id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) owner_id: UserId,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = books))]
pub(crate) struct NewBook<'a> {
    pub(crate) name: &'a str,
    pub(crate) slug: &'a str,
    pub(crate) owner_id: UserId,
}
