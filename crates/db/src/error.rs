//! The error type for the generated `db::rpc` layer.
//!
//! Server-only (it wraps `diesel::result::Error`). The API boundary maps it to
//! an HTTP status via [`Error::code`].

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub enum Error {
    /// A row addressed by id wasn't found in the caller's book.
    #[snafu(display("{entity} {id} not found"))]
    NotFound { entity: &'static str, id: String },

    /// Any underlying Diesel / database failure.
    #[snafu(display("database error: {source}"), context(false))]
    Query { source: diesel::result::Error },
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The HTTP status this maps to at the API boundary.
    pub fn code(&self) -> u16 {
        match self {
            Error::NotFound { .. } => 404,
            Error::Query { .. } => 500,
        }
    }
}
