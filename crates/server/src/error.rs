use {
    axum::{http::StatusCode, response::IntoResponse},
    dioxus::prelude::ServerFnError,
    snafu::prelude::*,
    webauthn_rs::prelude::WebauthnError,
};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Validation error: {msg}"))]
    Validation { msg: String },

    #[snafu(display("Login required"))]
    Unauthorized,

    #[snafu(display("Forbidden"))]
    Forbidden,

    #[snafu(display("{msg}"))]
    NotFound { msg: String },

    #[snafu(display("Database exhausted: {source}"))]
    DatabasePool {
        source: diesel_async::pooled_connection::deadpool::PoolError,
    },

    #[snafu(display("database error: {source}"), context(false))]
    Query { source: diesel::result::Error },

    #[snafu(display("session error: {source}"))]
    Session { source: ServerFnError },

    #[snafu(display("{source}"), context(false))]
    Db { source: db::Error },

    #[snafu(display("malformed host: {host}, expected base: {base}"))]
    MalformedHost { host: String, base: String },

    #[snafu(display("request is missing a Host header"))]
    MissingHost,

    #[snafu(display("internal error: {msg}"))]
    Internal { msg: String },

    #[snafu(display("webauthn: {source}"))]
    Webauthn { source: WebauthnError },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl Error {
    fn code(&self) -> StatusCode {
        match self {
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::NotFound { msg: _ } => StatusCode::NOT_FOUND,
            Error::Validation { msg: _ } => StatusCode::UNPROCESSABLE_ENTITY,
            Error::Query { source: _ }
            | Error::Session { source: _ }
            | Error::MalformedHost { host: _, base: _ }
            | Error::MissingHost
            | Error::Internal { msg: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::DatabasePool { source: _ } => StatusCode::SERVICE_UNAVAILABLE,
            Error::Db { source } => source.code(),
            Error::Webauthn { source: _ } => StatusCode::UNAUTHORIZED,
        }
    }
}

impl From<Error> for ServerFnError {
    fn from(value: Error) -> Self {
        ServerFnError::ServerError {
            message: value.to_string(),
            code: value.code().as_u16(),
            details: None,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (self.code(), self.to_string()).into_response()
    }
}

#[cfg(test)]
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Validation { msg: l_msg }, Self::Validation { msg: r_msg }) => l_msg == r_msg,
            (Self::NotFound { msg: l_msg }, Self::NotFound { msg: r_msg }) => l_msg == r_msg,
            (Self::DatabasePool { source: _ }, Self::DatabasePool { source: _ }) => true,
            (Self::Query { source: l_source }, Self::Query { source: r_source }) => {
                l_source == r_source
            }
            (Self::Session { source: l_source }, Self::Session { source: r_source }) => {
                l_source == r_source
            }
            (Self::Db { source: l_source }, Self::Db { source: r_source }) => l_source == r_source,
            (
                Self::MalformedHost {
                    host: l_host,
                    base: l_base,
                },
                Self::MalformedHost {
                    host: r_host,
                    base: r_base,
                },
            ) => l_host == r_host && l_base == r_base,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
