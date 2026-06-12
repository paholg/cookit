use {
    crate::{
        Timestamp,
        id::{BookId, UserId, UserRoleId},
    },
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
    pub updated_at: Timestamp,
    pub user_id: UserId,
    pub role: Role,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = user_roles))]
pub struct UserRoleNew {
    pub book_id: BookId,
    pub user_id: UserId,
    pub role: Role,
}
