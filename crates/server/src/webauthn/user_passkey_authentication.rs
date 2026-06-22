use {
    crate::webauthn::{UserPasskeyAuthenticationId, json_wrapper::JsonWrapper},
    db::{Timestamp, id::UserId, prelude::*, schema::user_passkey_authentications},
    diesel_async::AsyncPgConnection,
    serde::{Deserialize, Serialize},
    webauthn_rs::prelude::PasskeyAuthentication,
};

#[derive(Debug, Insertable)]
#[diesel(table_name = user_passkey_authentications)]
pub struct UserPasskeyAuthenticationCreate<'a> {
    pub user_id: UserId,
    #[diesel(serialize_as = JsonWrapper<PasskeyAuthentication>)]
    pub passkey_authentication: &'a PasskeyAuthentication,
}

#[derive(Debug, Clone, Serialize, Deserialize, HasQuery, Identifiable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPasskeyAuthentication {
    pub id: UserPasskeyAuthenticationId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub user_id: UserId,
    #[diesel(deserialize_as = JsonWrapper<PasskeyAuthentication>)]
    pub passkey_authentication: PasskeyAuthentication,
}

impl UserPasskeyAuthentication {
    pub async fn upsert(
        mut conn: &AsyncPgConnection,
        user_id: UserId,
        passkey_authentication: &PasskeyAuthentication,
    ) -> crate::Result<()> {
        UserPasskeyAuthenticationCreate {
            user_id,
            passkey_authentication,
        }
        .insert_into(user_passkey_authentications::table)
        .on_conflict(user_passkey_authentications::user_id)
        .do_update()
        .set(
            user_passkey_authentications::passkey_authentication
                .eq(JsonWrapper(passkey_authentication)),
        )
        .execute(&mut conn)
        .await?;

        Ok(())
    }

    pub async fn find_by_user(
        mut conn: &AsyncPgConnection,
        user_id: UserId,
    ) -> crate::Result<UserPasskeyAuthentication> {
        user_passkey_authentications::table
            .filter(user_passkey_authentications::user_id.eq(user_id))
            .first(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn delete(&self, mut conn: &AsyncPgConnection) -> diesel::result::QueryResult<()> {
        diesel::delete(
            user_passkey_authentications::table
                .filter(user_passkey_authentications::id.eq(self.id)),
        )
        .execute(&mut conn)
        .await?;

        Ok(())
    }
}
