use facet::Facet;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(Facet, Serialize, Deserialize, PartialEq, Eq, Clone, ThisError, Debug)]
#[repr(C)]
pub enum HttpError {
    // potentially external, have representation in shells
    // Note: must come first to preserve discriminant order on both sides of FFI
    #[error("URL parse error: {0}")]
    Url(String),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Timeout")]
    Timeout,

    // internal only, not generated or serialized
    #[error("HTTP error {code}: {message}")]
    #[serde(skip)]
    #[facet(skip)]
    Http {
        code: u16,
        message: String,
        body: Option<Vec<u8>>,
    },
    #[error("JSON serialization error: {0}")]
    #[serde(skip)]
    #[facet(skip)]
    Json(String),
}

impl From<http_types::Error> for HttpError {
    fn from(e: http_types::Error) -> Self {
        Self::Http {
            code: e.status().into(),
            message: e.to_string(),
            body: None,
        }
    }
}

impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<serde_json::Error> for HttpError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

impl From<url::ParseError> for HttpError {
    fn from(e: url::ParseError) -> Self {
        Self::Url(e.to_string())
    }
}

impl From<serde_qs::Error> for HttpError {
    fn from(e: serde_qs::Error) -> Self {
        Self::Json(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = HttpError::Http {
            code: 400,
            message: "Bad Request".to_string(),
            body: None,
        };
        assert_eq!(error.to_string(), "HTTP error 400: Bad Request");
    }

    #[test]
    fn http_code_is_plain_u16() {
        // The code field is a u16, so any valid status code literal works.
        let error = HttpError::Http {
            code: 404u16,
            message: "Not Found".to_string(),
            body: None,
        };
        assert_eq!(error.to_string(), "HTTP error 404: Not Found");
    }

    #[test]
    fn io_error_converts_to_io_variant() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let http_err = HttpError::from(io_err);
        assert!(matches!(http_err, HttpError::Io(_)));
        assert_eq!(http_err.to_string(), "IO error: file not found");
    }
}
