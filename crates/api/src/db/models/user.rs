use {
    crate::id::UserId,
    jiff::Timestamp,
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{prelude::*, schema::users};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
pub struct User {
    pub id: UserId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub updated_at: Timestamp,
    pub name: String,
    pub email: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = users))]
pub struct UserNew<'a> {
    pub email: &'a str,
    pub name: &'a str,
}
