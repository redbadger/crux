#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Error {
    message: String,
    code: Option<crate::http::StatusCode>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(code) = self.code {
            write!(f, "{}: {}", code, self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl Error {
    pub fn new(code: Option<crate::http::StatusCode>, message: impl Into<String>) -> Self {
        Error {
            message: message.into(),
            code,
        }
    }
}

impl From<crate::http::Error> for Error {
    fn from(e: crate::http::Error) -> Self {
        Error {
            message: e.to_string(),
            code: Some(e.status()),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error {
            message: e.to_string(),
            code: None,
        }
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error {
            message: e.to_string(),
            code: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::new(Some(crate::http::StatusCode::BadRequest), "Bad Request");
        assert_eq!(error.to_string(), "400: Bad Request");

        let error = Error::new(None, "internal server error");
        assert_eq!(error.to_string(), "internal server error");
    }
}
