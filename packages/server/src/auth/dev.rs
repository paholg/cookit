//! Dev-only login: two endpoints that upsert a fixed user / admin and issue a
//! session cookie. Intended for local development; activated by the
//! `dev-auth` cargo feature. Never compile this into a production build.

use anyhow::Result;
use axum::Router;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use cookie::time::Duration as CookieDuration;
use cookie::{Key, KeyError};
use std::sync::OnceLock;

use super::{
    SESSION_COOKIE, SESSION_TTL_DAYS, Session, cookie_builder, jar_from_headers, now,
    redirect_with_cookies, upsert_user,
};

// In dev-auth mode we always serve over plain HTTP (no TLS), so `Secure`
// cookies would be silently dropped.
const COOKIE_SECURE: bool = false;

const DEV_USER_SUB: &str = "__dev_user__";
const DEV_ADMIN_SUB: &str = "__dev_admin__";

/// Fixed signing key for dev session cookies. We're explicitly OK leaking
/// this — it's only used when the binary was compiled with `dev-auth`,
/// which is a non-default feature meant for local development.
const DEV_SESSION_KEY: &[u8; 64] =
    b"cookit-dev-auth-static-cookie-signing-key------------padding----";

pub(super) fn session_key() -> &'static Key {
    static KEY: OnceLock<Key> = OnceLock::new();
    KEY.get_or_init(|| {
        Key::try_from(&DEV_SESSION_KEY[..])
            .unwrap_or_else(|e: KeyError| panic!("dev session key must be at least 64 bytes: {e}"))
    })
}

pub(super) async fn router() -> Router {
    Router::new()
        .route("/auth/dev-login/user", post(login_user))
        .route("/auth/dev-login/admin", post(login_admin))
        .route("/auth/logout", post(logout))
}

async fn login_user() -> Result<Response, DevAuthError> {
    issue_session(DEV_USER_SUB, "dev-user@local", "Dev User", false).await
}

async fn login_admin() -> Result<Response, DevAuthError> {
    issue_session(DEV_ADMIN_SUB, "dev-admin@local", "Dev Admin", true).await
}

async fn issue_session(
    sub: &str,
    email: &str,
    name: &str,
    is_admin: bool,
) -> Result<Response, DevAuthError> {
    let user_id = upsert_user(sub, email, name, &[], is_admin).await?;

    let session = Session {
        user_id,
        exp: now() + SESSION_TTL_DAYS * 86400,
    };
    let session_value = serde_json::to_string(&session).expect("serialize session");

    let mut jar = cookie::CookieJar::new();
    jar.signed_mut(session_key()).add(
        cookie_builder(SESSION_COOKIE, session_value, COOKIE_SECURE)
            .max_age(CookieDuration::days(SESSION_TTL_DAYS))
            .build(),
    );

    Ok(redirect_with_cookies("/", &jar))
}

async fn logout(headers: axum::http::HeaderMap) -> Response {
    let mut jar = jar_from_headers(&headers);
    jar.signed_mut(session_key())
        .remove(cookie_builder(SESSION_COOKIE, String::new(), COOKIE_SECURE).build());
    redirect_with_cookies("/", &jar)
}

#[derive(Debug)]
struct DevAuthError(anyhow::Error);

impl From<anyhow::Error> for DevAuthError {
    fn from(e: anyhow::Error) -> Self {
        Self(e)
    }
}

impl IntoResponse for DevAuthError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:#}", self.0)).into_response()
    }
}
