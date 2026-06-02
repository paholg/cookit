use {dioxus::prelude::ServerFnError, snafu::prelude::*};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Validation error: {msg}"))]
    Validation { msg: String },
    #[snafu(display("Forbidden"))]
    Forbidden,

    #[cfg(feature = "server")]
    #[snafu(display("Database exhausted: {source}"))]
    DatabasePool {
        source: diesel_async::pooled_connection::deadpool::PoolError,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn code(&self) -> u16 {
        match self {
            Error::Validation { msg: _ } => 422,
            Error::Forbidden => 403,
            #[cfg(feature = "server")]
            Error::DatabasePool { source: _ } => 503,
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
