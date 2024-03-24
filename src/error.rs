//! Error definitions

/// Library result type
pub type Result<T> = std::result::Result<T, Error>;

/// Library errors
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum Error {
    // Basic errors
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
    #[error("Signature error: {0}")]
    Signature(String),
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

// impl From<k256::ecdsa::Error> for Error {
//     fn from(err: k256::ecdsa::Error) -> Self {
//         Error::Signature(err.to_string())
//     }
// }

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

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.to_string())
    }
}
