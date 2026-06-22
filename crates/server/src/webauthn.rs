mod client;
mod json_wrapper;
mod user_passkey;
mod user_passkey_authentication;
mod user_passkey_registration;

pub use client::{WebauthnClient, client};
use db::{
    id::{DraftId, Id, TablePrefix},
    table_id,
};

table_id! {
    pky, UserPasskeyId, UserPasskeyDraftId, UserPasskeyTable;
    pka, UserPasskeyAuthenticationId, UserPasskeyAuthenticationDraftId, UserPasskeyAuthenticationTable;
    pkr, UserPasskeyRegistrationId, UserPasskeyRegistrationDraftId, UserPasskeyRegistrationTable;
}
