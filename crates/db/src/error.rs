//! The error type for the generated `db::rpc` layer.
//!
//! Server-only (it wraps `diesel::result::Error`). The API boundary maps it to
//! an HTTP status via [`Error::code`].

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("no cookbook selected"))]
    NoBook,
    #[snafu(display("{entity} {id} not found"))]
    NotFound { entity: &'static str, id: String },

    #[snafu(display("database error: {source}"), context(false))]
    Query { source: diesel::result::Error },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The HTTP status this maps to at the API boundary.
    pub fn code(&self) -> u16 {
        match self {
            Error::NoBook => 404,
            Error::NotFound { .. } => 404,
            Error::Query { .. } => 500,
        }
    }
}
