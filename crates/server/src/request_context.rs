use {
    crate::{
        conn::{DbConn, get_conn},
        error::{ForbiddenSnafu, NotFoundSnafu, SessionSnafu, UnauthorizedSnafu},
        session::{self, AppAuthSession, AuthUserId},
    },
    axum::{
        extract::FromRequestParts,
        http::{StatusCode, request::Parts},
    },
    db::{id::BookId, models::user::AuthUser, rpc::RpcContext},
    diesel_async::AsyncPgConnection,
    dioxus::fullstack::FullstackContext,
    snafu::{OptionExt, ResultExt, ensure},
};

pub struct RequestContext {
    conn: DbConn,
    auth: AuthUser,
}

impl RequestContext {
    /// Log in as `user_id`.
    // TODO: Temporary
    pub async fn login(user_id: db::id::UserId) -> crate::Result<AuthUser> {
        let mut conn = get_conn().await?;
        let auth_user = session::load_auth_user(&mut conn, user_id).await?;
        let auth: AppAuthSession = FullstackContext::extract().await.context(SessionSnafu)?;
        auth.login_user(AuthUserId(Some(user_id)));
        auth.remember_user(true);

        Ok(auth_user)
    }

    /// Log in as the first user/book.
    // TODO: Temporary
    pub async fn login_first() -> crate::Result<AuthUser> {
        let mut conn = get_conn().await?;
        let auth_user = session::load_first_auth_user(&mut conn).await?;

        let auth: AppAuthSession = FullstackContext::extract().await.context(SessionSnafu)?;
        auth.login_user(AuthUserId(auth_user.user.as_ref().map(|u| u.id)));
        auth.remember_user(true);

        Ok(auth_user)
    }

    pub async fn logout() -> crate::Result<()> {
        let auth: AppAuthSession = FullstackContext::extract().await.context(SessionSnafu)?;
        auth.logout_user();

        Ok(())
    }

    /// Build a context for a specific user, no HTTP session involved (used by the
    /// seed binary).
    pub async fn load_for_user(mut conn: DbConn, user_id: db::id::UserId) -> crate::Result<Self> {
        let auth = session::load_auth_user(&mut conn, user_id).await?;
        Ok(Self { conn, auth })
    }

    pub fn book_id(&self) -> crate::Result<BookId> {
        let id = self
            .auth
            .book
            .as_ref()
            .context(NotFoundSnafu {
                msg: "No book!".to_string(),
            })?
            .id;

        Ok(id)
    }

    pub fn conn(&mut self) -> &mut DbConn {
        &mut self.conn
    }

    pub fn current_user(&self) -> &AuthUser {
        &self.auth
    }

    pub fn require_book(&self) -> crate::Result<()> {
        ensure!(self.auth.book.is_some(), UnauthorizedSnafu);

        Ok(())
    }

    pub fn require_admin(&self) -> crate::Result<()> {
        ensure!(self.auth.is_admin(), ForbiddenSnafu);

        Ok(())
    }
}

/// Built per request from the `AppAuthSession` that `AuthSessionLayer` puts in
/// the request extensions — which already holds the (cached) [`AuthUser`], so no
/// query runs here on a cache hit. Outside an HTTP request — Rust integration
/// tests, the seed binary — there is no such extension, so we fall back to the
/// first user/book, keeping non-request callers working without faking a session.
impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut conn = get_conn()
            .await
            .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

        let auth = match parts.extensions.get::<AppAuthSession>() {
            Some(auth) => auth.current_user.clone().unwrap_or_else(AuthUser::none),
            None => session::load_first_auth_user(&mut conn)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
        };

        Ok(Self { conn, auth })
    }
}

/// Lets the generated `db::rpc` operations run against a context's connection
/// and active book without `db` having to depend on this crate.
impl RpcContext for RequestContext {
    fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }

    fn book_id(&self) -> db::Result<BookId> {
        self.auth
            .book
            .as_ref()
            .map(|b| b.id)
            .context(db::error::NoBookSnafu)
    }
}
