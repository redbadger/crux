use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type TimeResult<T> = Result<T, TimeError>;

/// Error type for time operations
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum TimeError {
    #[error("invalid time")]
    InvalidTime,
    #[error("invalid Duration")]
    InvalidDuration,
    #[error("invalid Instant")]
    InvalidInstant,
}
