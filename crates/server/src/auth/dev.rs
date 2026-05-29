//! Dev-only login: a `<select>` of every row in `users`, populated via a
//! server function from the web client, plus a POST endpoint that issues a
//! session cookie for the chosen id. Activated by the `dev-auth` cargo
//! feature. Never compile this into a production build.

use anyhow::{Context, Result, anyhow};
use axum::Router;
use axum::extract::Form;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use cookie::time::Duration as CookieDuration;
use cookie::{Key, KeyError};
use serde::Deserialize;
use std::sync::OnceLock;
use types::DevUser;

use super::{
    SESSION_COOKIE, SESSION_TTL_DAYS, Session, cookie_builder, jar_from_headers, now,
    redirect_with_cookies,
};

// In dev-auth mode we always serve over plain HTTP (no TLS), so `Secure`
// cookies would be silently dropped.
const COOKIE_SECURE: bool = false;

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
        .route("/auth/dev-login", post(login_submit))
        .route("/auth/logout", post(logout))
}

#[derive(Debug, Deserialize)]
struct LoginForm {
    user_id: i64,
}

/// All users in the DB. Used to populate the dev login `<select>` in the
/// navbar via a server function.
pub async fn list_dev_users() -> Result<Vec<DevUser>> {
    let pool = crate::db::pool().await;
    let rows = sqlx::query!(
        r#"SELECT id as "id!: i64", name as "name!", is_admin as "is_admin!: bool"
           FROM users ORDER BY name COLLATE NOCASE"#,
    )
    .fetch_all(pool)
    .await
    .context("list_dev_users select")?;

    Ok(rows
        .into_iter()
        .map(|r| DevUser {
            id: r.id,
            name: r.name,
            is_admin: r.is_admin,
        })
        .collect())
}

async fn login_submit(Form(form): Form<LoginForm>) -> Result<Response, DevAuthError> {
    let pool = crate::db::pool().await;
    let exists = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM users WHERE id = ?) as "exists!: bool""#,
        form.user_id,
    )
    .fetch_one(pool)
    .await
    .context("login_submit user lookup")?;

    if !exists {
        return Err(anyhow!("user id {} not found — run `just seed`", form.user_id).into());
    }

    let session = Session {
        user_id: form.user_id,
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
