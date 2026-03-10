use facet::Facet;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for `KeyValue` operations
#[derive(Facet, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[cfg_attr(feature = "native_bridge", derive(uniffi::Enum))]
#[serde(rename_all = "camelCase")]
#[repr(C)]
pub enum KeyValueError {
    #[error("IO error: {message}")]
    Io { message: String },
    #[error("timeout")]
    Timeout,
    #[error("cursor not found")]
    CursorNotFound,
    #[error("other error: {message}")]
    Other { message: String },
}

pub type Result<T> = core::result::Result<T, KeyValueError>;
pub type StatusResult = Result<bool>;
pub type DataResult = Result<Option<Vec<u8>>>;
pub type ListResult = Result<(Vec<String>, u64)>;
