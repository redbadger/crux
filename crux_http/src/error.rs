use facet::Facet;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;

#[derive(Facet, Serialize, Deserialize, PartialEq, Eq, Clone, ThisError, Debug)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Enum))]
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
        #[facet(opaque)]
        code: http_types::StatusCode,
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

impl From<serde_qs::Error> for HttpError {
    fn from(e: serde_qs::Error) -> Self {
        HttpError::Json(e.to_string())
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
