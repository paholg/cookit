use {
    crate::{Timestamp, id::UserPasskeyId},
    serde::{Deserialize, Serialize},
};

/// Client-facing info from a UserPasskey.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasskeyInfo {
    pub id: UserPasskeyId,
    pub created_at: Timestamp,
}
