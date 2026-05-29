//! OIDC login flow: env-driven configuration, login/callback/logout HTTP
//! handlers, and session-cookie issuance on successful login.

use anyhow::{Context, Result, anyhow};
use axum::Router;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use cookie::time::Duration as CookieDuration;
use cookie::{CookieJar, Key};
use openidconnect::core::{CoreProviderMetadata, CoreResponseType};
use openidconnect::{
    AdditionalClaims, AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, reqwest,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::OnceCell;

use super::{
    SESSION_COOKIE, SESSION_TTL_DAYS, Session, cookie_builder, jar_from_headers, now,
    redirect_with_cookies,
};

const STATE_COOKIE: &str = "cookit_oidc_state";
const DEFAULT_ADMIN_GROUP: &str = "cookit_admin";
const DEFAULT_USER_GROUP: &str = "cookit_user";

/// All OIDC + session configuration, loaded once from env.
#[derive(Clone)]
pub(super) struct AuthConfig {
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_url: String,
    pub admin_group: String,
    pub user_group: String,
    pub cookie_secure: bool,
    pub key: Key,
}

impl AuthConfig {
    fn from_env() -> Result<Self> {
        let issuer_url = std::env::var("OIDC_ISSUER_URL").context("OIDC_ISSUER_URL not set")?;
        validate_url("OIDC_ISSUER_URL", &issuer_url)?;
        let client_id = std::env::var("OIDC_CLIENT_ID").context("OIDC_CLIENT_ID not set")?;
        let client_secret = read_secret("OIDC_CLIENT_SECRET")?;
        let redirect_url =
            std::env::var("OIDC_REDIRECT_URL").context("OIDC_REDIRECT_URL not set")?;
        validate_url("OIDC_REDIRECT_URL", &redirect_url)?;
        let admin_group =
            std::env::var("OIDC_ADMIN_GROUP").unwrap_or_else(|_| DEFAULT_ADMIN_GROUP.to_string());
        let user_group =
            std::env::var("OIDC_USER_GROUP").unwrap_or_else(|_| DEFAULT_USER_GROUP.to_string());
        let cookie_secure = std::env::var("SESSION_COOKIE_SECURE")
            .ok()
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        let secret_b64 = read_secret("SESSION_SECRET")?;
        let secret = base64_decode(&secret_b64).context("SESSION_SECRET must be base64")?;
        if secret.len() < 64 {
            return Err(anyhow!(
                "SESSION_SECRET must decode to at least 64 bytes, got {}",
                secret.len()
            ));
        }
        let key = Key::from(&secret);

        Ok(Self {
            issuer_url,
            client_id,
            client_secret,
            redirect_url,
            admin_group,
            user_group,
            cookie_secure,
            key,
        })
    }
}

static CONFIG: OnceCell<AuthConfig> = OnceCell::const_new();

pub(super) async fn config() -> &'static AuthConfig {
    CONFIG
        .get_or_init(|| async {
            AuthConfig::from_env().expect("failed to load auth configuration")
        })
        .await
}

/// `groups` claim on top of the standard OIDC claim set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct GroupsClaim {
    #[serde(default)]
    groups: Vec<String>,
}

impl AdditionalClaims for GroupsClaim {}

type OidcClient = openidconnect::Client<
    GroupsClaim,
    openidconnect::core::CoreAuthDisplay,
    openidconnect::core::CoreGenderClaim,
    openidconnect::core::CoreJweContentEncryptionAlgorithm,
    openidconnect::core::CoreJsonWebKey,
    openidconnect::core::CoreAuthPrompt,
    openidconnect::StandardErrorResponse<openidconnect::core::CoreErrorResponseType>,
    openidconnect::StandardTokenResponse<
        openidconnect::IdTokenFields<
            GroupsClaim,
            openidconnect::EmptyExtraTokenFields,
            openidconnect::core::CoreGenderClaim,
            openidconnect::core::CoreJweContentEncryptionAlgorithm,
            openidconnect::core::CoreJwsSigningAlgorithm,
        >,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::StandardTokenIntrospectionResponse<
        openidconnect::EmptyExtraTokenFields,
        openidconnect::core::CoreTokenType,
    >,
    openidconnect::core::CoreRevocableToken,
    openidconnect::StandardErrorResponse<openidconnect::RevocationErrorResponseType>,
    EndpointSet,      // HasAuthUrl
    EndpointNotSet,   // HasDeviceAuthUrl
    EndpointNotSet,   // HasIntrospectionUrl
    EndpointNotSet,   // HasRevocationUrl
    EndpointMaybeSet, // HasTokenUrl
    EndpointMaybeSet, // HasUserInfoUrl
>;

/// Shared axum state for the auth router.
#[derive(Clone)]
struct AuthState {
    cfg: Arc<AuthConfig>,
    http: reqwest::Client,
}

async fn build_oidc_client(state: &AuthState, issuer_url: &str) -> Result<OidcClient> {
    let issuer = IssuerUrl::new(issuer_url.to_string()).context("invalid issuer URL")?;
    let metadata = CoreProviderMetadata::discover_async(issuer, &state.http)
        .await
        .context("OIDC discovery failed")?;
    let redirect =
        RedirectUrl::new(state.cfg.redirect_url.clone()).context("invalid OIDC_REDIRECT_URL")?;

    Ok(openidconnect::Client::from_provider_metadata(
        metadata,
        ClientId::new(state.cfg.client_id.clone()),
        Some(ClientSecret::new(state.cfg.client_secret.clone())),
    )
    .set_redirect_uri(redirect))
}

/// Build the axum router for `/auth/*` endpoints.
pub(super) async fn router() -> Router {
    let cfg = config().await.clone();

    let http = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build reqwest client for OIDC");

    let state = AuthState {
        cfg: Arc::new(cfg),
        http,
    };

    Router::new()
        .route("/auth/login", get(login))
        .route("/auth/callback", get(callback))
        .route("/auth/logout", post(logout))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct LoginQuery {
    #[serde(default)]
    return_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OidcState {
    csrf: String,
    nonce: String,
    pkce_verifier: String,
    return_to: String,
}

async fn login(
    State(state): State<AuthState>,
    Query(query): Query<LoginQuery>,
) -> Result<Response, AuthError> {
    let client = build_oidc_client(&state, &state.cfg.issuer_url)
        .await
        .map_err(AuthError::from)?;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (auth_url, csrf, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("groups".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let state_payload = OidcState {
        csrf: csrf.secret().clone(),
        nonce: nonce.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
        return_to: sanitize_return_to(query.return_to.as_deref()),
    };
    let value = serde_json::to_string(&state_payload).expect("serialize oidc state");

    let mut jar = CookieJar::new();
    jar.signed_mut(&state.cfg.key).add(
        cookie_builder(STATE_COOKIE, value, state.cfg.cookie_secure)
            .max_age(CookieDuration::minutes(10))
            .build(),
    );

    Ok(redirect_with_cookies(auth_url.as_str(), &jar))
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn callback(
    State(state): State<AuthState>,
    headers: axum::http::HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Result<Response, AuthError> {
    if let Some(err) = query.error {
        return Err(AuthError::Provider(format!(
            "provider returned error: {err}"
        )));
    }
    let code = query
        .code
        .ok_or_else(|| AuthError::Bad("missing code".into()))?;
    let csrf_from_idp = query
        .state
        .ok_or_else(|| AuthError::Bad("missing state".into()))?;

    let mut jar = jar_from_headers(&headers);
    let saved_cookie = jar
        .signed(&state.cfg.key)
        .get(STATE_COOKIE)
        .ok_or_else(|| AuthError::Bad("missing or invalid state cookie".into()))?;
    let oidc_state: OidcState = serde_json::from_str(saved_cookie.value())
        .map_err(|_| AuthError::Bad("malformed state cookie".into()))?;

    if oidc_state.csrf != csrf_from_idp {
        return Err(AuthError::Bad("state mismatch".into()));
    }

    let client = build_oidc_client(&state, &state.cfg.issuer_url).await?;
    let token_response = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| AuthError::Provider(format!("exchange_code: {e}")))?
        .set_pkce_verifier(PkceCodeVerifier::new(oidc_state.pkce_verifier))
        .request_async(&state.http)
        .await
        .map_err(|e| AuthError::Provider(format!("token request failed: {e:?}")))?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| AuthError::Provider("ID token missing from response".into()))?;
    let verifier = client.id_token_verifier();
    let claims = id_token
        .claims(&verifier, &Nonce::new(oidc_state.nonce))
        .map_err(|e| AuthError::Provider(format!("id_token verify: {e}")))?;

    let sub = claims.subject().as_str().to_string();
    let email = claims
        .email()
        .map(|e| e.as_str().to_string())
        .unwrap_or_default();
    let name = claims
        .preferred_username()
        .map(|n| n.as_str().to_string())
        .or_else(|| {
            claims
                .name()
                .and_then(|m| m.get(None).map(|n| n.as_str().to_string()))
        })
        .unwrap_or_else(|| email.clone());
    let groups: Vec<String> = claims.additional_claims().groups.clone();

    let admin_group = &state.cfg.admin_group;
    let user_group = &state.cfg.user_group;
    let is_admin = groups.iter().any(|g| g == admin_group);
    let is_member = is_admin || groups.iter().any(|g| g == user_group);

    // Clear the state cookie regardless of outcome.
    jar.signed_mut(&state.cfg.key)
        .remove(cookie_builder(STATE_COOKIE, String::new(), state.cfg.cookie_secure).build());

    if !is_member {
        return Err(AuthError::Forbidden(format!(
            "Login rejected: account `{sub}` is not in `{admin_group}` or `{user_group}`. \
             Groups: {groups:?}."
        )));
    }

    let user_id = upsert_user(&sub, &email, &name, &groups, is_admin)
        .await
        .map_err(AuthError::from)?;

    let session = Session {
        user_id,
        exp: now() + SESSION_TTL_DAYS * 86400,
    };
    let session_value = serde_json::to_string(&session).expect("serialize session");
    jar.signed_mut(&state.cfg.key).add(
        cookie_builder(SESSION_COOKIE, session_value, state.cfg.cookie_secure)
            .max_age(CookieDuration::days(SESSION_TTL_DAYS))
            .build(),
    );

    Ok(redirect_with_cookies(&oidc_state.return_to, &jar))
}

async fn logout(
    State(state): State<AuthState>,
    headers: axum::http::HeaderMap,
) -> Result<Response, AuthError> {
    let mut jar = jar_from_headers(&headers);
    jar.signed_mut(&state.cfg.key)
        .remove(cookie_builder(SESSION_COOKIE, String::new(), state.cfg.cookie_secure).build());
    Ok(redirect_with_cookies("/", &jar))
}

// -- helpers -----------------------------------------------------------------

fn sanitize_return_to(value: Option<&str>) -> String {
    // Only allow same-origin relative paths to prevent open-redirects.
    match value {
        Some(v) if v.starts_with('/') && !v.starts_with("//") => v.to_string(),
        _ => "/".to_string(),
    }
}

fn validate_url(name: &str, value: &str) -> Result<()> {
    url::Url::parse(value)
        .with_context(|| format!("{name} is not a valid URL: `{value}`"))
        .map(|_| ())
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| anyhow!("base64 decode: {e}"))
}

/// Read a secret value from either `${name}` (literal) or `${name}_FILE` (path).
/// The file form is intended for systems like agenix where secrets are
/// materialized as files on disk and shouldn't appear in the process
/// environment table.
fn read_secret(name: &str) -> Result<String> {
    if let Ok(v) = std::env::var(name) {
        return Ok(v);
    }
    let file_var = format!("{name}_FILE");
    let path = std::env::var(&file_var)
        .with_context(|| format!("{name} not set (also tried {file_var})"))?;
    std::fs::read_to_string(&path)
        .map(|s| s.trim_end().to_string())
        .with_context(|| format!("reading {file_var} = {path}"))
}

#[derive(Debug)]
enum AuthError {
    Bad(String),
    Forbidden(String),
    Provider(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for AuthError {
    fn from(e: anyhow::Error) -> Self {
        AuthError::Internal(e)
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AuthError::Bad(m) => (StatusCode::BAD_REQUEST, m),
            AuthError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            AuthError::Provider(m) => (StatusCode::BAD_GATEWAY, m),
            // Echo the error chain — "internal error" with no context is
            // exactly the silent-failure mode we want to avoid.
            AuthError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:#}")),
        };
        (status, msg).into_response()
    }
}

async fn upsert_user(
    sub: &str,
    email: &str,
    name: &str,
    groups: &[String],
    is_admin: bool,
) -> Result<i64> {
    let pool = crate::db_sqlite::pool().await;
    let groups_csv = groups.join(",");
    let is_admin_int: i64 = if is_admin { 1 } else { 0 };
    let row = sqlx::query!(
        r#"INSERT INTO users (oidc_sub, email, name, groups, is_admin)
           VALUES (?1, ?2, ?3, ?4, ?5)
           ON CONFLICT(oidc_sub) DO UPDATE SET
               email = excluded.email,
               name = excluded.name,
               groups = excluded.groups,
               is_admin = excluded.is_admin
           RETURNING id as "id!: i64""#,
        sub,
        email,
        name,
        groups_csv,
        is_admin_int,
    )
    .fetch_one(pool)
    .await
    .context("upsert_user")?;
    Ok(row.id)
}
