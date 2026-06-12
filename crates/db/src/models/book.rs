use {
    crate::{
        Name, Slug, Timestamp,
        id::{BookId, UserId},
    },
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
    pub updated_at: Timestamp,
    pub name: Name,
    pub slug: Slug,
    pub owner_id: UserId,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = books))]
pub struct BookNew {
    pub name: Name,
    pub slug: Slug,
    pub owner_id: UserId,
}
