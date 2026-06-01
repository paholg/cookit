use {dioxus::server::ServerFnError, snafu::prelude::*};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Validation error: {msg}"))]
    Validation { msg: String },
    #[snafu(display("Forbidden"))]
    Forbidden,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn code(&self) -> u16 {
        match self {
            Error::Validation { msg: _ } => 422,
            Error::Forbidden => 403,
        }
    }
}

impl From<Error> for ServerFnError {
    fn from(value: Error) -> Self {
        match value {
            _ => ServerFnError::ServerError {
                message: value.to_string(),
                code: value.code(),
                details: None,
            },
        }
    }
}
