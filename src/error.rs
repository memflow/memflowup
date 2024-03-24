//! Error definitions

/// Library result type
pub type Result<T> = std::result::Result<T, Error>;

/// Library errors
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum Error {
    // Basic errors
    #[error("Unknown error: {0}")]
    Unknown(String),
    #[error("IO error: {0}")]
    IO(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Already exists: {0}")]
    AlreadyExists(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    // External crate error forwards
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("Registry error: {0}")]
    Registry(String),
    #[error("Signature error: {0}")]
    Signature(String),
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Unknown(err.to_owned())
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(err: std::convert::Infallible) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IO(err.to_string())
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(value: std::str::Utf8Error) -> Self {
        Error::Parse(format!("Unable to parse utf8: {}", value))
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Error::Parse(err.to_string())
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Error::Parse(err.to_string())
    }
}

impl From<crates_io_api::Error> for Error {
    fn from(err: crates_io_api::Error) -> Self {
        Error::Http(err.to_string())
    }
}

impl From<memflow_registry_client::Error> for Error {
    fn from(err: memflow_registry_client::Error) -> Self {
        Error::Registry(err.to_string())
    }
}
