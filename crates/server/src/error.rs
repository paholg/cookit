use {dioxus::prelude::ServerFnError, snafu::prelude::*};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Validation error: {msg}"))]
    Validation { msg: String },

    #[snafu(display("Forbidden"))]
    Forbidden,

    #[snafu(display("Database exhausted: {source}"))]
    DatabasePool {
        source: diesel_async::pooled_connection::deadpool::PoolError,
    },

    #[snafu(display("{source}"), context(false))]
    Db { source: db::error::Error },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn code(&self) -> u16 {
        match self {
            Error::Validation { msg: _ } => 422,
            Error::Forbidden => 403,
            Error::DatabasePool { source: _ } => 503,
            Error::Db { source } => source.code(),
        }
    }
}

impl From<Error> for ServerFnError {
    fn from(value: Error) -> Self {
        ServerFnError::ServerError {
            message: value.to_string(),
            code: value.code(),
            details: None,
        }
    }
}
