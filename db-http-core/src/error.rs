use std::fmt;

#[derive(Debug)]
pub enum Error {
    Http(String),
    Serialization(serde_json::Error),
    Database(String),
    Config(String),
    NoRows,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(msg) => write!(f, "HTTP error: {msg}"),
            Error::Serialization(err) => write!(f, "Serialization error: {err}"),
            Error::Database(msg) => write!(f, "Database error: {msg}"),
            Error::Config(msg) => write!(f, "Config error: {msg}"),
            Error::NoRows => write!(f, "Query returned no rows"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Serialization(err) => Some(err),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err)
    }
}
