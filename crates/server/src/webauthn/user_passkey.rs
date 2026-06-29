use {
    crate::webauthn::json_wrapper::JsonWrapper,
    base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD},
    base64urlsafedata::HumanBinaryData,
    db::{
        Name, Timestamp,
        id::{UserId, UserPasskeyId},
        models::passkey::PasskeyInfo,
        prelude::*,
        schema::user_passkeys,
    },
    diesel_async::AsyncPgConnection,
    serde::{Deserialize, Serialize},
    webauthn_rs::prelude::Passkey,
};

#[derive(Debug, Insertable)]
#[diesel(table_name = user_passkeys)]
pub struct UserPasskeyCreate<'a> {
    pub user_id: UserId,
    pub credential_id: &'a str,
    pub name: &'a Name,
    #[diesel(serialize_as = JsonWrapper<Passkey>)]
    pub passkey: &'a Passkey,
}

pub async fn list_passkeys(
    conn: &AsyncPgConnection,
    user_id: UserId,
) -> crate::Result<Vec<PasskeyInfo>> {
    let passkeys = UserPasskey::list(conn, user_id).await?;

    Ok(passkeys.iter().map(UserPasskey::info).collect())
}

pub async fn delete_passkey(
    conn: &AsyncPgConnection,
    user_id: UserId,
    id: UserPasskeyId,
) -> crate::Result<()> {
    UserPasskey::delete(conn, id, user_id).await
}

fn cred_id_to_string(cred_id: &HumanBinaryData) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(cred_id)
}

#[derive(Debug, Clone, Serialize, Deserialize, HasQuery, Identifiable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPasskey {
    pub id: UserPasskeyId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub deleted_at: Option<Timestamp>,
    pub user_id: UserId,
    pub credential_id: String,
    #[diesel(deserialize_as = JsonWrapper<Passkey>)]
    pub passkey: Passkey,
    pub name: Name,
}

impl UserPasskey {
    pub fn info(&self) -> PasskeyInfo {
        PasskeyInfo {
            id: self.id,
            name: self.name.clone(),
            created_at: self.created_at,
        }
    }

    pub async fn list(mut conn: &AsyncPgConnection, user_id: UserId) -> crate::Result<Vec<Self>> {
        let rows = user_passkeys::table
            .filter(user_passkeys::user_id.eq(user_id))
            .filter(user_passkeys::deleted_at.is_null())
            .load(&mut conn)
            .await?;
        Ok(rows)
    }

    pub async fn create(
        mut conn: &AsyncPgConnection,
        user_id: UserId,
        name: &Name,
        passkey: &Passkey,
    ) -> diesel::result::QueryResult<()> {
        let cred_id = passkey.cred_id();
        let cred_id_base64 = cred_id_to_string(cred_id);

        UserPasskeyCreate {
            user_id,
            credential_id: &cred_id_base64,
            name,
            passkey,
        }
        .insert_into(user_passkeys::table)
        .execute(&mut conn)
        .await?;

        Ok(())
    }

    pub async fn find_by_cred_id(
        mut conn: &AsyncPgConnection,
        credential_id: &HumanBinaryData,
    ) -> crate::Result<Self> {
        let cred_id = cred_id_to_string(credential_id);
        user_passkeys::table
            .filter(user_passkeys::credential_id.eq(&cred_id))
            .filter(user_passkeys::deleted_at.is_null())
            .first(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn update_passkey(
        mut conn: &AsyncPgConnection,
        id: UserPasskeyId,
        passkey: Passkey,
    ) -> crate::Result<()> {
        diesel::update(user_passkeys::table)
            .filter(user_passkeys::id.eq(id))
            .filter(user_passkeys::deleted_at.is_null())
            .set(user_passkeys::passkey.eq(JsonWrapper(passkey)))
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    async fn delete(
        mut conn: &AsyncPgConnection,
        id: UserPasskeyId,
        user_id: UserId,
    ) -> crate::Result<()> {
        let now = Timestamp::new(jiff::Timestamp::now());

        diesel::update(user_passkeys::table)
            .filter(user_passkeys::id.eq(id))
            .filter(user_passkeys::user_id.eq(user_id))
            .filter(user_passkeys::deleted_at.is_null())
            .set(user_passkeys::deleted_at.eq(now))
            .execute(&mut conn)
            .await?;

        Ok(())
    }
}
