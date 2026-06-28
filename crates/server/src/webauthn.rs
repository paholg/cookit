mod client;
mod json_wrapper;
mod user_passkey;
mod user_passkey_authentication;
mod user_passkey_registration;

use db::{
    id::{DraftId, Id, TablePrefix},
    table_id,
};
pub use {
    client::{WebauthnClient, client},
    user_passkey::{delete_passkey, list_passkeys},
};

table_id! {
    pka, UserPasskeyAuthenticationId, UserPasskeyAuthenticationDraftId, UserPasskeyAuthenticationTable;
    pkr, UserPasskeyRegistrationId, UserPasskeyRegistrationDraftId, UserPasskeyRegistrationTable;
}
