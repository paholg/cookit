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
    webauthn_rs::prelude::{
        CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
        RequestChallengeResponse,
    },
};

table_id! {
    pky, UserPasskeyId, UserPasskeyDraftId, UserPasskeyTable;
    pka, UserPasskeyAuthenticationId, UserPasskeyAuthenticationDraftId, UserPasskeyAuthenticationTable;
    pkr, UserPasskeyRegistrationId, UserPasskeyRegistrationDraftId, UserPasskeyRegistrationTable;
}
