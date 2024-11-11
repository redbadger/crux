use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, ThisError, Debug)]
pub enum HttpError {
    #[error("HTTP error {code}: {message}")]
    #[serde(skip)]
    Http {
        code: http_types::StatusCode,
        message: String,
        body: Option<Vec<u8>>,
    },
    #[error("JSON serialisation error: {0}")]
    #[serde(skip)]
    Json(String),
    #[error("URL parse error: {0}")]
    Url(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Timeout")]
    Timeout,
}

impl From<http_types::Error> for HttpError {
    fn from(e: http_types::Error) -> Self {
        HttpError::Http {
            code: e.status(),
            message: e.to_string(),
            body: None,
        }
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(e: serde_json::Error) -> Self {
        HttpError::Json(e.to_string())
    }
}

impl From<url::ParseError> for HttpError {
    fn from(e: url::ParseError) -> Self {
        HttpError::Url(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = HttpError::Http {
            code: http_types::StatusCode::BadRequest,
            message: "Bad Request".to_string(),
            body: None,
        };
        assert_eq!(error.to_string(), "HTTP error 400: Bad Request");
    }
}
