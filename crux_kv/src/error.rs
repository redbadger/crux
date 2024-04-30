use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type KeyValueResult<T> = Result<T, KeyValueError>;

/// Error type for KeyValue operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum KeyValueError {
    #[error("IO error: {message}")]
    Io { message: String },
    #[error("timeout")]
    Timeout,
    #[error("other error: {message}")]
    OtherError { message: String },
}
