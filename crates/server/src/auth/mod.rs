//! Authentication: session cookies, login handlers, and helpers for server
//! functions to look up the current user.
//!
//! The login backend is selected at compile time via the `dev-auth` cargo
//! feature:
//! - OFF (default, production): OIDC against an env-configured provider — see
//!   [`oidc`].
//! - ON (local development only): simple "Log in as user" / "Log in as admin"
//!   buttons that upsert fixed users — see [`dev`]. Never enable this in
//!   production.

use anyhow::{Context, Result};
use axum::Router;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use cookie::time::OffsetDateTime;
use cookie::{Cookie, CookieJar, Key, SameSite};
use dioxus::fullstack::FullstackContext;
use dioxus::prelude::ServerFnError;
use serde::{Deserialize, Serialize};

#[cfg(feature = "dev-auth")]
mod dev;
// `oidc` is always compiled so sqlx::query! macros in this module are picked
// up by `cargo sqlx prepare`
#[cfg_attr(feature = "dev-auth", allow(dead_code))]
mod oidc;

#[cfg(feature = "dev-auth")]
pub use dev::list_dev_users;

pub(crate) const SESSION_COOKIE: &str = "cookit_session";
pub(crate) const SESSION_TTL_DAYS: i64 = 30;

/// Snapshot of the authenticated user, loaded fresh from the DB per request.
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub is_admin: bool,
}

/// Build the axum router for `/auth/*` endpoints.
pub async fn router() -> Router {
    #[cfg(feature = "dev-auth")]
    {
        dev::router().await
    }
    #[cfg(not(feature = "dev-auth"))]
    {
        oidc::router().await
    }
}

/// Look up the current user from the session cookie attached to the
/// in-flight server function request.
pub async fn current_user() -> Option<CurrentUser> {
    let key = session_key().await;
    let headers: axum::http::HeaderMap = FullstackContext::extract().await.ok()?;
    let jar = jar_from_headers(&headers);
    let cookie = jar.signed(key).get(SESSION_COOKIE)?;
    let session: Session = serde_json::from_str(cookie.value()).ok()?;
    if session.exp <= now() {
        return None;
    }
    // load_user(session.user_id).await.ok().flatten()
    todo!()
}

pub async fn require_user() -> Result<CurrentUser, ServerFnError> {
    current_user()
        .await
        .ok_or_else(|| status_err(StatusCode::UNAUTHORIZED, "login required"))
}

pub async fn require_admin() -> Result<CurrentUser, ServerFnError> {
    let user = require_user().await?;
    if !user.is_admin {
        return Err(status_err(StatusCode::FORBIDDEN, "admin only"));
    }
    Ok(user)
}

// -- shared internals --------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Session {
    pub user_id: i64,
    pub exp: i64,
}

pub(crate) async fn session_key() -> &'static Key {
    #[cfg(feature = "dev-auth")]
    {
        dev::session_key()
    }
    #[cfg(not(feature = "dev-auth"))]
    {
        &oidc::config().await.key
    }
}

// pub(crate) async fn load_user(id: i64) -> Result<Option<CurrentUser>> {
//     let pool = crate::db_sqlite::pool().await;
//     let row = sqlx::query!(
//         r#"SELECT id as "id!: i64", name as "name!", email as "email!",
//                   is_admin as "is_admin!: bool"
//            FROM users WHERE id = ?"#,
//         id,
//     )
//     .fetch_optional(pool)
//     .await
//     .context("load_user select")?;

//     Ok(row.map(|r| CurrentUser {
//         id: r.id,
//         name: r.name,
//         email: r.email,
//         is_admin: r.is_admin,
//     }))
// }

pub(crate) fn jar_from_headers(headers: &axum::http::HeaderMap) -> CookieJar {
    let mut jar = CookieJar::new();
    for cookie_header in headers.get_all(header::COOKIE) {
        let Ok(s) = cookie_header.to_str() else {
            continue;
        };
        for raw in s.split(';') {
            if let Ok(c) = Cookie::parse(raw.trim().to_string()) {
                jar.add_original(c);
            }
        }
    }
    jar
}

pub(crate) fn cookie_builder(
    name: &'static str,
    value: String,
    secure: bool,
) -> cookie::CookieBuilder<'static> {
    Cookie::build((name, value))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
}

pub(crate) fn redirect_with_cookies(location: &str, jar: &CookieJar) -> Response {
    let mut response = Redirect::to(location).into_response();
    for cookie in jar.delta() {
        if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
            response.headers_mut().append(header::SET_COOKIE, value);
        }
    }
    response
}

pub(crate) fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}

pub(crate) fn status_err(status: StatusCode, message: &str) -> ServerFnError {
    ServerFnError::ServerError {
        message: message.to_string(),
        code: status.as_u16(),
        details: None,
    }
}
