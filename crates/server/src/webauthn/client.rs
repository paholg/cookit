use {
    crate::{
        config,
        error::WebauthnSnafu,
        user,
        webauthn::{
            user_passkey::UserPasskey, user_passkey_authentication::UserPasskeyAuthentication,
            user_passkey_registration::UserPasskeyRegistration,
        },
    },
    db::{Name, id::UserId},
    diesel_async::{AsyncConnection, AsyncPgConnection},
    snafu::ResultExt,
    std::sync::LazyLock,
    webauthn_rs::prelude::*,
};

static WEBAUTHN_CLIENT: LazyLock<WebauthnClient> = LazyLock::new(WebauthnClient::new);

pub fn client() -> &'static WebauthnClient {
    &WEBAUTHN_CLIENT
}

pub struct WebauthnClient {
    webauthn: Webauthn,
}

impl WebauthnClient {
    fn new() -> Self {
        let config = config::config();

        let webauthn = WebauthnBuilder::new(config.webauth_rp_id(), &config.webauthn_origin())
            .unwrap()
            .rp_name("CookIt!")
            .allow_subdomains(true)
            .build()
            .unwrap();

        Self { webauthn }
    }

    pub async fn start_registration(
        &self,
        conn: &AsyncPgConnection,
        user_id: UserId,
    ) -> crate::Result<CreationChallengeResponse> {
        let (user, passkeys) =
            tokio::try_join!(user::find(conn, user_id), UserPasskey::list(conn, user_id))?;

        let credential_ids: Vec<CredentialID> = passkeys
            .into_iter()
            .map(|up| up.passkey.cred_id().to_owned())
            .collect();

        let (ccr, skr) = self
            .webauthn
            .start_passkey_registration(
                user_id.into(),
                user.email.as_str(),
                &user.name,
                Some(credential_ids),
            )
            .context(WebauthnSnafu)?;

        UserPasskeyRegistration::upsert(conn, user_id, &skr).await?;

        Ok(ccr)
    }

    pub async fn finish_registration(
        &self,
        conn: &mut AsyncPgConnection,
        user_id: UserId,
        name: &Name,
        reg: &RegisterPublicKeyCredential,
    ) -> crate::Result<()> {
        let upr = UserPasskeyRegistration::find_by_user(conn, user_id).await?;

        let passkey = self
            .webauthn
            .finish_passkey_registration(reg, &upr.passkey_registration)
            .context(WebauthnSnafu)?;

        conn.transaction(async |conn| {
            tokio::try_join!(
                UserPasskey::create(conn, user_id, name, &passkey),
                upr.delete(conn)
            )?;

            diesel::result::QueryResult::Ok(())
        })
        .await?;

        Ok(())
    }

    pub async fn start_passkey_authentication(
        &self,
        conn: &AsyncPgConnection,
        user_id: UserId,
    ) -> crate::Result<RequestChallengeResponse> {
        let passkeys: Vec<Passkey> = UserPasskey::list(conn, user_id)
            .await?
            .into_iter()
            .map(|up| up.passkey)
            .collect();

        let (rcr, pa) = self
            .webauthn
            .start_passkey_authentication(&passkeys)
            .context(WebauthnSnafu)?;

        UserPasskeyAuthentication::upsert(conn, user_id, &pa).await?;

        Ok(rcr)
    }

    pub async fn finish_passkey_authentication(
        &self,
        conn: &AsyncPgConnection,
        user_id: UserId,
        reg: &PublicKeyCredential,
    ) -> crate::Result<()> {
        let upa = UserPasskeyAuthentication::find_by_user(conn, user_id).await?;

        let result = self
            .webauthn
            .finish_passkey_authentication(reg, &upa.passkey_authentication)
            .context(WebauthnSnafu)?;

        if result.needs_update() {
            let cred_id = result.cred_id();
            let user_passkey = UserPasskey::find_by_cred_id(conn, cred_id).await?;
            let mut passkey = user_passkey.passkey;
            passkey.update_credential(&result);

            UserPasskey::update_passkey(conn, user_passkey.id, passkey).await?;
        }

        upa.delete(conn).await?;

        Ok(())
    }
}
