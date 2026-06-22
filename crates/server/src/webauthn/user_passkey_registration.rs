use {
    crate::webauthn::{UserPasskeyRegistrationId, json_wrapper::JsonWrapper},
    db::{Timestamp, id::UserId, prelude::*, schema::user_passkey_registrations},
    diesel_async::AsyncPgConnection,
    serde::{Deserialize, Serialize},
    webauthn_rs::prelude::PasskeyRegistration,
};

#[derive(Debug, Insertable)]
#[diesel(table_name = user_passkey_registrations)]
pub struct UserPasskeyRegistrationCreate<'a> {
    pub user_id: UserId,
    #[diesel(serialize_as = JsonWrapper<PasskeyRegistration>)]
    pub passkey_registration: &'a PasskeyRegistration,
}

#[derive(Debug, Clone, Serialize, Deserialize, HasQuery, Identifiable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPasskeyRegistration {
    pub id: UserPasskeyRegistrationId,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub user_id: UserId,
    #[diesel(deserialize_as = JsonWrapper<PasskeyRegistration>)]
    pub passkey_registration: PasskeyRegistration,
}

impl UserPasskeyRegistration {
    pub async fn upsert(
        mut conn: &AsyncPgConnection,
        user_id: UserId,
        passkey_registration: &PasskeyRegistration,
    ) -> crate::Result<()> {
        UserPasskeyRegistrationCreate {
            user_id,
            passkey_registration,
        }
        .insert_into(user_passkey_registrations::table)
        .on_conflict(user_passkey_registrations::user_id)
        .do_update()
        .set(user_passkey_registrations::passkey_registration.eq(JsonWrapper(passkey_registration)))
        .execute(&mut conn)
        .await?;

        Ok(())
    }

    pub async fn find_by_user(
        mut conn: &AsyncPgConnection,
        user_id: UserId,
    ) -> crate::Result<UserPasskeyRegistration> {
        user_passkey_registrations::table
            .filter(user_passkey_registrations::user_id.eq(user_id))
            .first(&mut conn)
            .await
            .map_err(Into::into)
    }

    pub async fn delete(&self, mut conn: &AsyncPgConnection) -> diesel::result::QueryResult<()> {
        diesel::delete(
            user_passkey_registrations::table.filter(user_passkey_registrations::id.eq(self.id)),
        )
        .execute(&mut conn)
        .await?;

        Ok(())
    }
}
