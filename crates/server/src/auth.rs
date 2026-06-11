//! Cookie-based sessions: a signed cookie carrying the active `user_role_id`.
//!
//! This is deliberately minimal — the previous OIDC auth was removed and will
//! be reintroduced later. The cookie is *signed* (not merely stored) because
//! the one invariant we enforce is the `book_id` boundary: a user must not be
//! able to hand-edit the cookie to act as a `user_role` in another book.

use {
    axum::http::{HeaderMap, HeaderValue, header},
    cookie::{Cookie, CookieJar, Key, SameSite, time::Duration},
    db::id::UserRoleId,
    dioxus::fullstack::FullstackContext,
    serde::{Deserialize, Serialize},
    std::{
        sync::LazyLock,
        time::{SystemTime, UNIX_EPOCH},
    },
};

const SESSION_COOKIE: &str = "cookit_session";
const SESSION_TTL_DAYS: i64 = 30;

// We serve plain HTTP locally, so a `Secure` cookie would be silently dropped.
const COOKIE_SECURE: bool = false;

/// Fallback signing key used when `SESSION_SECRET` is unset. Fine to leak: it
/// only matters for local development, and a real deployment must set the env.
/// `Key::from` requires at least 64 bytes.
const DEV_KEY: &[u8] =
    b"cookit-dev-session-signing-key-not-for-production-use-please-set-SESSION_SECRET";

/// HMAC key for the signed session cookie. Built from `SESSION_SECRET` (≥64
/// bytes) if set, otherwise a fixed development key.
static KEY: LazyLock<Key> = LazyLock::new(|| match std::env::var("SESSION_SECRET") {
    Ok(secret) => {
        assert!(
            secret.len() >= 64,
            "SESSION_SECRET must be at least 64 bytes, got {}",
            secret.len()
        );
        Key::from(secret.as_bytes())
    }
    Err(_) => {
        tracing::warn!(
            "SESSION_SECRET not set; using a fixed insecure development signing key. Set \
             SESSION_SECRET (>=64 bytes) outside local development."
        );
        Key::from(DEV_KEY)
    }
});

#[derive(Serialize, Deserialize)]
struct SessionPayload {
    user_role_id: UserRoleId,
    /// Unix seconds; the cookie is ignored past this even if its Max-Age hasn't
    /// elapsed (e.g. a clock-skewed client).
    exp: i64,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The `user_role_id` from the current request's session cookie, if present and
/// unexpired. Returns `None` when there's no cookie or it fails verification.
pub async fn current_role_id() -> Option<UserRoleId> {
    let headers: HeaderMap = FullstackContext::extract().await.ok()?;
    let jar = jar_from_headers(&headers);
    let cookie = jar.signed(&KEY).get(SESSION_COOKIE)?;

    let payload: SessionPayload = serde_json::from_str(cookie.value()).ok()?;
    if payload.exp <= now() {
        return None;
    }

    Some(payload.user_role_id)
}

/// Issue a session cookie for `user_role_id` on the in-flight response.
pub fn set_session_cookie(user_role_id: UserRoleId) {
    let payload = SessionPayload {
        user_role_id,
        exp: now() + SESSION_TTL_DAYS * 86_400,
    };
    let value = serde_json::to_string(&payload).expect("serialize session");

    let mut jar = CookieJar::new();
    jar.signed_mut(&KEY).add(
        Cookie::build((SESSION_COOKIE, value))
            .path("/")
            .http_only(true)
            .same_site(SameSite::Lax)
            .secure(COOKIE_SECURE)
            .max_age(Duration::days(SESSION_TTL_DAYS))
            .build(),
    );

    for cookie in jar.delta() {
        add_set_cookie(cookie.to_string());
    }
}

/// Clear the session cookie on the in-flight response.
pub fn clear_session_cookie() {
    let removal = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(COOKIE_SECURE)
        .max_age(Duration::seconds(0))
        .build();

    add_set_cookie(removal.to_string());
}

fn add_set_cookie(value: String) {
    if let (Some(ctx), Ok(header_value)) =
        (FullstackContext::current(), HeaderValue::from_str(&value))
    {
        ctx.add_response_header(header::SET_COOKIE, header_value);
    }
}

fn jar_from_headers(headers: &HeaderMap) -> CookieJar {
    let mut jar = CookieJar::new();

    for value in headers.get_all(header::COOKIE) {
        let Ok(s) = value.to_str() else {
            continue;
        };
        for raw in s.split(';') {
            if let Ok(cookie) = Cookie::parse(raw.trim().to_owned()) {
                jar.add_original(cookie);
            }
        }
    }

    jar
}
