use {
    crate::conn::{DbConn, DbPool, POOL},
    async_trait::async_trait,
    axum_session::{DatabaseError, DatabasePool, SessionConfig, SessionLayer, SessionStore},
    axum_session_auth::{AuthConfig, AuthSessionLayer, Authentication},
    db::{
        Timestamp,
        id::UserId,
        models::user::User,
        schema::{sessions, users},
    },
    diesel::{
        dsl::{exists, now},
        prelude::*,
        select,
    },
    diesel_async::RunQueryDsl,
    serde::{Deserialize, Serialize},
    std::fmt,
};

/// ID used by `axum_session_auth`.
#[derive(Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AuthUserId(pub Option<UserId>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user: Option<User>,
}

impl fmt::Display for AuthUserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(id) => id.fmt(f),
            None => f.write_str("<anonymous>"),
        }
    }
}

pub type CookitAuthSession =
    axum_session_auth::AuthSession<AuthUser, AuthUserId, DieselSessionPool, DieselSessionPool>;

pub async fn install(router: axum::Router) -> axum::Router {
    let pool = DieselSessionPool::new();

    let base_domain = &crate::config::config().base_domain;
    let config = SessionConfig::default().with_cookie_domain(base_domain.clone());

    let store = SessionStore::new(Some(pool.clone()), config)
        .await
        .expect("failed to build session store");

    router
        .layer(
            AuthSessionLayer::<AuthUser, AuthUserId, DieselSessionPool, DieselSessionPool>::new(
                Some(pool),
            )
            .with_config(AuthConfig::<AuthUserId>::default()),
        )
        .layer(SessionLayer::new(store))
}

/// Newtype wrapper around our database pool, so we can implement `axum_session::DatabasePool`.
#[derive(Clone)]
pub struct DieselSessionPool(pub DbPool);

impl DieselSessionPool {
    pub fn new() -> Self {
        Self(POOL.clone())
    }

    async fn conn(&self) -> Result<DbConn, DatabaseError> {
        self.0
            .get()
            .await
            .map_err(|e| DatabaseError::GenericAcquire(e.to_string()))
    }
}

impl std::fmt::Debug for DieselSessionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DieselSessionPool").finish_non_exhaustive()
    }
}

#[async_trait]
impl DatabasePool for DieselSessionPool {
    // NOTE: We manage the table with diesel, so ignore the table_name and don't
    // create the table here.
    async fn initiate(&self, _table_name: &str) -> Result<(), DatabaseError> {
        Ok(())
    }

    async fn count(&self, _table_name: &str) -> Result<i64, DatabaseError> {
        let mut conn = self.conn().await?;

        sessions::table
            .count()
            .get_result(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericSelectError(e.to_string()))
    }

    async fn store(
        &self,
        id: &str,
        session: &str,
        expires: i64,
        _table_name: &str,
    ) -> Result<(), DatabaseError> {
        let expires_at = Timestamp::new(
            jiff::Timestamp::from_second(expires)
                .map_err(|e| DatabaseError::GenericInsertError(e.to_string()))?,
        );

        let mut conn = self.conn().await?;

        diesel::insert_into(sessions::table)
            .values((
                sessions::id.eq(id),
                sessions::session.eq(session),
                sessions::expires_at.eq(expires_at),
            ))
            .on_conflict(sessions::id)
            .do_update()
            .set((
                sessions::session.eq(session),
                sessions::expires_at.eq(expires_at),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericInsertError(e.to_string()))?;

        Ok(())
    }

    async fn load(&self, id: &str, _table_name: &str) -> Result<Option<String>, DatabaseError> {
        let mut conn = self.conn().await?;

        sessions::table
            .filter(sessions::id.eq(id))
            .filter(sessions::expires_at.gt(now))
            .select(sessions::session)
            .first::<String>(&mut conn)
            .await
            .optional()
            .map_err(|e| DatabaseError::GenericSelectError(e.to_string()))
    }

    async fn delete_one_by_id(&self, id: &str, _table_name: &str) -> Result<(), DatabaseError> {
        let mut conn = self.conn().await?;

        diesel::delete(sessions::table.filter(sessions::id.eq(id)))
            .execute(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericDeleteError(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, id: &str, _table_name: &str) -> Result<bool, DatabaseError> {
        let mut conn = self.conn().await?;

        select(exists(sessions::table.filter(sessions::id.eq(id))))
            .get_result(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericSelectError(e.to_string()))
    }

    async fn delete_by_expiry(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self.conn().await?;

        diesel::delete(sessions::table.filter(sessions::expires_at.lt(now)))
            .returning(sessions::id)
            .get_results(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericDeleteError(e.to_string()))
    }

    async fn delete_all(&self, _table_name: &str) -> Result<(), DatabaseError> {
        let mut conn = self.conn().await?;

        diesel::delete(sessions::table)
            .execute(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericDeleteError(e.to_string()))?;

        Ok(())
    }

    async fn get_ids(&self, _table_name: &str) -> Result<Vec<String>, DatabaseError> {
        let mut conn = self.conn().await?;

        sessions::table
            .filter(sessions::expires_at.gt(now))
            .select(sessions::id)
            .load(&mut conn)
            .await
            .map_err(|e| DatabaseError::GenericSelectError(e.to_string()))
    }

    fn auto_handles_expiry(&self) -> bool {
        false
    }
}

#[async_trait]
impl Authentication<AuthUser, AuthUserId, DieselSessionPool> for AuthUser {
    async fn load_user(
        userid: AuthUserId,
        pool: Option<&DieselSessionPool>,
    ) -> anyhow::Result<AuthUser> {
        use anyhow::Context;

        let pool = pool.context("AuthSessionLayer was created without a pool")?;
        let user_id = userid.0.context("no user id in session")?;

        let mut conn = pool.0.get().await?;
        let user: Option<User> = users::table
            .find(user_id)
            .first(&mut conn)
            .await
            .optional()?;

        Ok(AuthUser { user })
    }

    fn is_authenticated(&self) -> bool {
        self.user.is_some()
    }

    fn is_active(&self) -> bool {
        self.user
            .as_ref()
            .map(|u| u.deleted_at.is_none())
            .unwrap_or(false)
    }

    fn is_anonymous(&self) -> bool {
        self.user.is_none()
    }
}
