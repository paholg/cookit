use {
    crate::id::UserId,
    jiff::Timestamp,
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use crate::db::{prelude::*, schema::users};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(HasQuery, Identifiable, AsChangeset))]
#[cfg_attr(feature = "server", diesel(check_for_backend(diesel::pg::Pg)))]
pub(crate) struct User {
    pub(crate) id: UserId,
    #[cfg_attr(feature = "server", diesel(serialize_as = jiff_diesel::Timestamp, deserialize_as = jiff_diesel::Timestamp))]
    pub(crate) updated_at: Timestamp,
    pub(crate) email: String,
    pub(crate) name: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "server", derive(Insertable))]
#[cfg_attr(feature = "server", diesel(table_name = users))]
pub(crate) struct NewUser<'a> {
    pub(crate) email: &'a str,
    pub(crate) name: &'a str,
}
