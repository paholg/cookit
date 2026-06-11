use {
    crate::id::{BookId, UserId, UserRoleId},
    serde::{Deserialize, Serialize},
};
#[cfg(feature = "server")]
use {
    crate::{
        models::{book::Book, user::User},
        schema::user_roles,
    },
    diesel::prelude::*,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(diesel_derive_enum::DbEnum))]
#[cfg_attr(
    feature = "server",
    ExistingTypePath = "crate::schema::sql_types::Role"
)]
pub enum Role {
    Admin,
    User,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, Associations))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
#[cfg_attr(feature = "server", diesel(belongs_to(Book)))]
#[cfg_attr(feature = "server", diesel(belongs_to(User)))]
pub struct UserRole {
    pub id: UserRoleId,
    pub book_id: BookId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: jiff::Timestamp,
    pub user_id: UserId,
    pub role: Role,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::NullableTimestamp, deserialize_as = jiff_diesel::NullableTimestamp))]
    pub deleted_at: Option<jiff::Timestamp>,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub created_at: jiff::Timestamp,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = user_roles))]
pub struct UserRoleNew {
    pub book_id: BookId,
    pub user_id: UserId,
    pub role: Role,
}
