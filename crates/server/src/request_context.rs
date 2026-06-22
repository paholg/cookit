use {
    crate::{
        Error, book,
        config::config,
        conn::{DbConn, get_conn},
        error::{
            ForbiddenSnafu, InternalSnafu, MissingHostSnafu, NotFoundSnafu, SessionSnafu,
            UnauthorizedSnafu,
        },
        session::{AuthUserId, CookitAuthSession},
        user_role,
    },
    axum::{
        extract::FromRequestParts,
        http::{header, request::Parts},
    },
    db::{
        id::{BookId, UserId},
        models::{
            user::{Current, User},
            user_role::UserRole,
        },
        prelude::*,
        rpc::RpcContext,
        schema::{books, user_roles, users},
    },
    diesel_async::AsyncPgConnection,
    dioxus::fullstack::FullstackContext,
    snafu::{OptionExt, ResultExt, ensure},
};

pub struct RequestContext {
    conn: DbConn,
    pub current: Current,
}

impl RequestContext {
    // TODO: Passkeys
    pub async fn login_as(&mut self, user: User) -> crate::Result<Current> {
        let auth: CookitAuthSession = FullstackContext::extract().await.context(SessionSnafu)?;
        auth.login_user(AuthUserId(Some(user.id)));
        auth.remember_user(true);

        let book = book::load_home_book(&mut self.conn, user.id).await?;

        let user_role = match &book {
            Some(book) => user_role::try_find(&mut self.conn, user.id, book.id).await?,
            None => None,
        };

        Ok(Current {
            user: Some(user),
            book,
            role: user_role.map(|ur| ur.role),
        })
    }

    // TODO: Passkeys
    pub async fn login_first(&mut self) -> crate::Result<Current> {
        let user: User = users::table
            .order_by(users::id.asc())
            .select(User::as_returning())
            .first(&mut self.conn)
            .await?;

        self.login_as(user).await
    }

    pub async fn logout() -> crate::Result<()> {
        let auth: CookitAuthSession = FullstackContext::extract().await.context(SessionSnafu)?;
        auth.logout_user();

        Ok(())
    }

    // NOTE: This is currently only used in `seed`. Find a better way.
    pub async fn load_for_user(mut conn: DbConn, user_id: UserId) -> crate::Result<Self> {
        let user: User = users::table.find(user_id).first(&mut conn).await?;

        let user_role: Option<UserRole> = UserRole::belonging_to(&user)
            .order_by(user_roles::id.asc())
            .first(&mut conn)
            .await
            .optional()?;

        let book = match &user_role {
            Some(r) => Some(books::table.find(r.book_id).first(&mut conn).await?),
            None => None,
        };

        Ok(Self {
            conn,
            current: Current {
                user: Some(user),
                book,
                role: user_role.map(|ur| ur.role),
            },
        })
    }

    pub fn book_id(&self) -> crate::Result<BookId> {
        let id = self
            .current
            .book
            .as_ref()
            .context(NotFoundSnafu {
                msg: "No book for this host".to_string(),
            })?
            .id;

        Ok(id)
    }

    pub fn conn(&mut self) -> &mut DbConn {
        &mut self.conn
    }

    pub fn require_user(&self) -> crate::Result<&User> {
        let Some(user) = &self.current.user else {
            return Err(Error::Unauthorized);
        };

        Ok(user)
    }

    pub fn require_book(&self) -> crate::Result<()> {
        ensure!(
            self.current.book.is_some(),
            NotFoundSnafu {
                msg: "No book for this host".to_string(),
            }
        );
        ensure!(self.current.role.is_some(), UnauthorizedSnafu);

        Ok(())
    }

    pub fn require_admin(&self) -> crate::Result<()> {
        self.require_book()?;

        let is_admin = self.current.is_admin();
        ensure!(is_admin, ForbiddenSnafu);

        Ok(())
    }
}

impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = crate::Error;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut conn = get_conn().await?;

        let auth = parts
            .extensions
            .get::<CookitAuthSession>()
            .context(InternalSnafu {
                msg: "missing auth middleware",
            })?;

        let user = auth.current_user.clone().and_then(|au| au.user);

        let host = request_host(parts)?;
        let slug = config().book_slug(host)?;

        let book = match slug {
            Some(slug) => book::find_by_slug(&mut conn, &slug).await?,
            None => None,
        };

        let user_role = match (&user, &book) {
            (Some(user), Some(book)) => user_role::try_find(&mut conn, user.id, book.id).await?,
            _ => None,
        };

        Ok(Self {
            conn,
            current: Current {
                user,
                book,
                role: user_role.map(|ur| ur.role),
            },
        })
    }
}

fn request_host(parts: &Parts) -> crate::Result<&str> {
    parts
        .headers
        .get("x-forwarded-host")
        .or_else(|| parts.headers.get(header::HOST))
        .and_then(|h| h.to_str().ok())
        .context(MissingHostSnafu)
}

/// Lets the generated `db::rpc` operations run against a context's connection
/// and active book without `db` having to depend on this crate.
impl RpcContext for RequestContext {
    fn conn(&mut self) -> &mut AsyncPgConnection {
        &mut self.conn
    }

    fn book_id(&self) -> db::Result<BookId> {
        self.current
            .book
            .as_ref()
            .map(|b| b.id)
            .context(db::error::NoBookSnafu)
    }
}
