use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, ThisError, Debug)]
pub enum Error {
    #[error("HTTP error {0}")]
    Http(HttpError),
    #[error("JSON serialisation error: {0}")]
    Json(String),
    #[error("URL parse error: {0}")]
    Url(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Timeout")]
    Timeout,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq, ThisError)]
#[error("{code}: {message}")]
pub struct HttpError {
    pub message: String,
    pub code: crate::http::StatusCode,
    pub body: Option<Vec<u8>>,
}

impl HttpError {
    pub fn new(
        code: crate::http::StatusCode,
        message: impl Into<String>,
        body: Option<Vec<u8>>,
    ) -> Self {
        Self {
            message: message.into(),
            code,
            body,
        }
    }
}

impl From<crate::http::Error> for Error {
    fn from(e: crate::http::Error) -> Self {
        Error::Http(HttpError {
            message: e.to_string(),
            code: e.status(),
            body: None,
        })
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::Url(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::Http(HttpError::new(
            crate::http::StatusCode::BadRequest,
            "Bad Request",
            None,
        ));
        assert_eq!(error.to_string(), "HTTP error 400: Bad Request");
    }
}
