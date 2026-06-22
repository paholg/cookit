#[cfg(feature = "server")]
use {crate::schema::users, diesel::prelude::*};
use {
    crate::{
        Email, Name, Timestamp,
        id::UserId,
        models::{book::Book, user_role::Role},
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Current {
    pub user: Option<User>,
    pub book: Option<Book>,
    pub role: Option<Role>,
}

impl Current {
    pub fn none() -> Self {
        Self {
            user: None,
            book: None,
            role: None,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role.is_some_and(|r| r.is_admin())
    }

    pub fn is_logged_in(&self) -> bool {
        self.user.is_some()
    }
}
