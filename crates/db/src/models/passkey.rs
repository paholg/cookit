use {
    crate::{Name, Timestamp, id::UserPasskeyId},
    serde::{Deserialize, Serialize},
};

/// Client-facing info from a UserPasskey.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: UserPasskeyId,
    pub name: Name,
    pub created_at: Timestamp,
}
