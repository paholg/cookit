//! Server-side session storage.

use {
    crate::conn::{DbConn, DbPool, POOL},
    anyhow::Context as _,
    async_trait::async_trait,
    axum_session::{DatabaseError, DatabasePool, SessionConfig, SessionLayer, SessionStore},
    axum_session_auth::{AuthConfig, AuthSessionLayer, Authentication},
    db::{
        Timestamp,
        id::UserId,
        models::{
            book::Book,
            user::{AuthUser, User},
            user_role::UserRole,
        },
        schema::{books, sessions, user_roles, users},
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

/// Id used by `axum_session_auth`.
#[derive(Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct AuthUserId(pub Option<UserId>);

impl fmt::Display for AuthUserId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(id) => id.fmt(f),
            None => f.write_str("<anonymous>"),
        }
    }
}

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

pub type AppAuthSession =
    axum_session_auth::AuthSession<AuthUser, AuthUserId, DieselSessionPool, DieselSessionPool>;

pub async fn install(router: axum::Router) -> axum::Router {
    let pool = DieselSessionPool::new();

    let config = SessionConfig::default();
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

/// Load a user's identity — user + first `user_role` + that role's book.
/// Returns `None` if the user doesn't exist or has no role/book yet.
pub(crate) async fn load_auth_user(conn: &mut DbConn, user_id: UserId) -> crate::Result<AuthUser> {
    let user: Option<User> = users::table.find(user_id).first(conn).await.optional()?;

    let Some(user) = user else {
        return Ok(AuthUser::none());
    };

    let role: Option<UserRole> = UserRole::belonging_to(&user)
        .order_by(user_roles::id.asc())
        .first(conn)
        .await
        .optional()?;
    let Some(role) = role else {
        return Ok(AuthUser {
            user: Some(user),
            role: None,
            book: None,
        });
    };

    let book: Book = books::table.find(role.book_id).first(conn).await?;

    Ok(AuthUser {
        user: Some(user),
        book: Some(book),
        role: Some(role),
    })
}

/// The first user/book/role, for callers with no HTTP session (Rust integration
/// tests, the seed binary).
pub(crate) async fn load_first_auth_user(conn: &mut DbConn) -> crate::Result<AuthUser> {
    let role: UserRole = user_roles::table
        .order_by(user_roles::id.asc())
        .first(conn)
        .await?;

    let user: User = users::table.find(role.user_id).first(conn).await?;
    let book: Book = books::table.find(role.book_id).first(conn).await?;

    Ok(AuthUser {
        user: Some(user),
        book: Some(book),
        role: Some(role),
    })
}

#[async_trait]
impl Authentication<AuthUser, AuthUserId, DieselSessionPool> for AuthUser {
    async fn load_user(
        userid: AuthUserId,
        pool: Option<&DieselSessionPool>,
    ) -> anyhow::Result<AuthUser> {
        let pool = pool.context("AuthSessionLayer was created without a pool")?;
        let user_id = userid.0.context("no user id in session")?;

        let mut conn = pool.0.get().await?;
        load_auth_user(&mut conn, user_id).await.map_err(Into::into)
    }

    fn is_authenticated(&self) -> bool {
        self.role.is_some()
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
