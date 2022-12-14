#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Error {
    message: String,
    code: Option<crate::http::StatusCode>,
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
