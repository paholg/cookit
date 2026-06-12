#[cfg(feature = "server")]
use {crate::schema::users, diesel::prelude::*};
use {
    crate::{
        Email, Name, Timestamp,
        id::{BookId, UserId},
        models::user_role::Role,
    },
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct User {
    pub id: UserId,
    pub updated_at: Timestamp,
    pub name: Name,
    pub email: Email,
    pub deleted_at: Option<Timestamp>,
    pub created_at: Timestamp,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = users))]
pub struct UserNew {
    pub email: Email,
    pub name: Name,
}

/// The logged-in user as the client sees it: identity plus the active book and
/// role, flattened from the server-side session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: UserId,
    pub book_id: BookId,
    pub name: Name,
    pub email: Email,
    pub role: Role,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}
