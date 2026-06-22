use {http::StatusCode, snafu::Snafu};

#[derive(Debug, Snafu, PartialEq)]
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
    pub fn code(&self) -> StatusCode {
        match self {
            Error::NoBook => StatusCode::NOT_FOUND,
            Error::NotFound { .. } => StatusCode::NOT_FOUND,
            Error::Query { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
