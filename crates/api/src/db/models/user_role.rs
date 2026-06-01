use {
    crate::id::{BookId, UserId, UserRoleId},
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{
    models::{book::Book, user::User},
    prelude::*,
    schema::user_roles,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(diesel_derive_enum::DbEnum))]
#[cfg_attr(
    feature = "server",
    ExistingTypePath = "crate::db::schema::sql_types::Role"
)]
pub(crate) enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "server",
    derive(HasQuery, Identifiable, AsChangeset, Associations)
)]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(User)))]
pub(crate) struct UserRole {
    pub(crate) id: UserRoleId,
    pub(crate) book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: jiff::Timestamp,
    pub(crate) user_id: UserId,
    pub(crate) role: Role,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = user_roles))]
pub(crate) struct NewUserRole {
    pub(crate) book_id: BookId,
    pub(crate) user_id: UserId,
    pub(crate) role: Role,
}
