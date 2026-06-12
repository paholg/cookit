use {
    crate::{
        auth,
        conn::{DbConn, get_conn},
        error::ForbiddenSnafu,
    },
    db::{
        id::{BookId, UserRoleId},
        models::{
            book::Book,
            user::{CurrentUser, User},
            user_role::{Role, UserRole},
        },
        rpc::RpcContext,
        schema::{books, user_roles, users},
    },
    diesel::{BelongingToDsl, ExpressionMethods, OptionalExtension, QueryDsl},
    diesel_async::{AsyncPgConnection, RunQueryDsl},
    dioxus::{fullstack::FullstackContext, prelude::ServerFnError},
    snafu::ensure,
};

pub struct Session {
    conn: DbConn,
    book: Book,
    user: User,
    role: UserRole,
}

impl Session {
    /// Resolve the session for the current request from its signed cookie.
    ///
    /// Outside any HTTP request context — Rust integration tests, scripts —
    /// there is no cookie, so we fall back to the first user/book. That keeps
    /// non-request callers working without having to fake a session.
    pub async fn from_request() -> anyhow::Result<Option<Self>> {
        let conn = get_conn().await?;

        if FullstackContext::current().is_none() {
            return Ok(Some(Self::load_first(conn).await?));
        }

        let Some(role_id) = auth::current_role_id().await else {
            return Ok(None);
        };

        Self::load_for_role(conn, role_id).await
    }

    /// Like [`Session::from_request`], but errors with 401 when there is no
    /// valid session. Use this in routes that require a logged-in user.
    pub async fn require() -> Result<Self, ServerFnError> {
        match Self::from_request().await? {
            Some(session) => Ok(session),
            None => Err(ServerFnError::ServerError {
                message: "login required".to_string(),
                code: 401,
                details: None,
            }),
        }
    }

    /// Log in as a specific `user_role` and issue its session cookie.
    pub async fn login(user_role_id: UserRoleId) -> Result<Self, ServerFnError> {
        let conn = get_conn().await?;
        let session = Self::load_for_role(conn, user_role_id)
            .await?
            .ok_or_else(|| ServerFnError::ServerError {
                message: format!("user_role {user_role_id} not found"),
                code: 404,
                details: None,
            })?;

        auth::set_session_cookie(user_role_id);
        Ok(session)
    }

    /// Log in as the first user/book and issue its session cookie. A dev
    /// convenience until real login is restored.
    pub async fn login_first() -> Result<Self, ServerFnError> {
        let conn = get_conn().await?;
        let session = Self::load_first(conn).await?;

        auth::set_session_cookie(session.role.id);
        Ok(session)
    }

    async fn load_first(mut conn: DbConn) -> anyhow::Result<Self> {
        let user: User = users::table
            .order_by(users::id.asc())
            .first(&mut conn)
            .await?;

        let book: Book = books::table
            .order_by(books::id.asc())
            .first(&mut conn)
            .await?;

        let role: UserRole = UserRole::belonging_to(&user)
            .filter(user_roles::book_id.eq(book.id))
            .first(&mut conn)
            .await?;

        Ok(Self {
            conn,
            book,
            user,
            role,
        })
    }

    pub async fn load_for_role(
        mut conn: DbConn,
        role_id: UserRoleId,
    ) -> anyhow::Result<Option<Self>> {
        let role: Option<UserRole> = user_roles::table
            .find(role_id)
            .first(&mut conn)
            .await
            .optional()?;

        let Some(role) = role else {
            return Ok(None);
        };

        let user: User = users::table.find(role.user_id).first(&mut conn).await?;
        let book: Book = books::table.find(role.book_id).first(&mut conn).await?;

        Ok(Some(Self {
            conn,
            book,
            user,
            role,
        }))
    }

    pub fn book_id(&self) -> BookId {
        self.book.id
    }

    pub fn conn(&mut self) -> &mut DbConn {
        &mut self.conn
    }

    pub fn current_user(&self) -> CurrentUser {
        CurrentUser {
            id: self.user.id,
            book_id: self.book.id,
            name: self.user.name.clone(),
            email: self.user.email.clone(),
            role: self.role.role,
        }
    }

    pub fn require_admin(&self) -> crate::Result<()> {
        ensure!(self.role.role == Role::Admin, ForbiddenSnafu);
        Ok(())
    }
}

/// Lets the generated `db::rpc` operations run against a session's connection
/// and book without `db` having to depend on this crate.
impl RpcContext for Session {
    fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }

    fn book_id(&self) -> BookId {
        self.book.id
    }
}
