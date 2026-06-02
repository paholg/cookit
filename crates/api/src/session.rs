#[cfg(feature = "server")]
use {
    crate::{
        auth,
        db::{
            conn::{DbConn, get_conn},
            models::{book::Book, user::User, user_role::UserRole},
            schema::{books, user_roles, users},
        },
        error::ForbiddenSnafu,
        id::UserRoleId,
    },
    diesel::{BelongingToDsl, ExpressionMethods, OptionalExtension, QueryDsl},
    diesel_async::RunQueryDsl,
    dioxus::{fullstack::FullstackContext, prelude::ServerFnError},
    snafu::ensure,
};
use {
    crate::{
        db::models::user_role::Role,
        id::{BookId, UserId},
    },
    serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
pub struct Session {
    conn: DbConn,
    book: Book,
    user: User,
    role: UserRole,
}

#[cfg(feature = "server")]
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

    async fn load_for_role(mut conn: DbConn, role_id: UserRoleId) -> anyhow::Result<Option<Self>> {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: UserId,
    pub book_id: BookId,
    pub name: String,
    pub email: String,
    pub role: Role,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}
