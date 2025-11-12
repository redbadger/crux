pub mod value;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::error::KeyValueError;
pub use value::*;

/// Supported operations
#[derive(Facet, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum KeyValueOperation {
    /// Read bytes stored under a key
    Get { key: String },
    /// Write bytes under a key
    Set {
        key: String,
        #[serde(with = "serde_bytes")]
        value: Vec<u8>,
    },
    /// Remove a key and its value
    Delete { key: String },
    /// Test if a key exists
    Exists { key: String },
    // List keys that start with a prefix, starting at the cursor
    ListKeys {
        /// The prefix to list keys for, or an empty string to list all keys
        prefix: String,
        /// The cursor to start listing from, or 0 to start from the beginning.
        /// If there are more keys to list, the response will include a new cursor.
        /// If there are no more keys, the response will include a cursor of 0.
        /// The cursor is opaque to the caller, and should be passed back to the
        /// `ListKeys` operation to continue listing keys.
        /// If the cursor is not found for the specified prefix, the response will include
        /// a `KeyValueError::CursorNotFound` error.
        cursor: u64,
    },
}

impl std::fmt::Debug for KeyValueOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyValueOperation::Get { key } => f.debug_struct("Get").field("key", key).finish(),
            KeyValueOperation::Set { key, value } => {
                let body_repr = if let Ok(s) = std::str::from_utf8(value) {
                    if s.len() < 50 {
                        format!("\"{s}\"")
                    } else {
                        format!("\"{}\"...", s.chars().take(50).collect::<String>())
                    }
                } else {
                    format!("<binary data - {} bytes>", value.len())
                };
                f.debug_struct("Set")
                    .field("key", key)
                    .field("value", &format_args!("{body_repr}"))
                    .finish()
            }
            KeyValueOperation::Delete { key } => {
                f.debug_struct("Delete").field("key", key).finish()
            }
            KeyValueOperation::Exists { key } => {
                f.debug_struct("Exists").field("key", key).finish()
            }
            KeyValueOperation::ListKeys { prefix, cursor } => f
                .debug_struct("ListKeys")
                .field("prefix", prefix)
                .field("cursor", cursor)
                .finish(),
        }
    }
}

/// The result of an operation on the store.
///
/// Note: we can't use [`core::result::Result`] here because it is not currently
/// supported across the FFI boundary, when using `typegen` or `facet_typegen`.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum KeyValueResult {
    Ok { response: KeyValueResponse },
    Err { error: KeyValueError },
}

#[derive(Facet, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(C)]
pub enum KeyValueResponse {
    /// Response to a `KeyValueOperation::Get`,
    /// returning the value stored under the key, which may be empty
    Get { value: Value },
    /// Response to a `KeyValueOperation::Set`,
    /// returning the value that was previously stored under the key, may be empty
    Set { previous: Value },
    /// Response to a `KeyValueOperation::Delete`,
    /// returning the value that was previously stored under the key, may be empty
    Delete { previous: Value },
    /// Response to a `KeyValueOperation::Exists`,
    /// returning whether the key is present in the store
    Exists { is_present: bool },
    /// Response to a `KeyValueOperation::ListKeys`,
    /// returning a list of keys that start with the prefix, and a cursor to continue listing
    /// if there are more keys
    ///
    /// Note: the cursor is 0 if there are no more keys
    ListKeys {
        keys: Vec<String>,
        /// The cursor to continue listing keys, or 0 if there are no more keys.
        /// If the cursor is not found for the specified prefix, the response should instead
        /// include a `KeyValueError::CursorNotFound` error.
        next_cursor: u64,
    },
}

impl Operation for KeyValueOperation {
    type Output = KeyValueResult;

    #[cfg(feature = "typegen")]
    fn register_types(
        generator: &mut crux_core::type_generation::serde::TypeGen,
    ) -> crux_core::type_generation::serde::Result {
        generator.register_type::<KeyValueResponse>()?;
        generator.register_type::<KeyValueError>()?;
        generator.register_type::<Value>()?;
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }
}

impl KeyValueResult {
    /// Converts a [`KeyValueResult`] into a [`Result`]
    /// # Errors
    /// Passes any errors from the underlying [`KeyValueError`] to the returned `Result`.
    /// # Panics
    /// Panics if the [`KeyValueResult`] is not a [`KeyValueResponse::Get`].
    pub fn unwrap_get(self) -> Result<Option<Vec<u8>>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Get { value } => Ok(value.into()),
                _ => {
                    panic!("attempt to convert KeyValueResponse other than Get to Option<Vec<u8>>")
                }
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    /// Converts a [`KeyValueResult`] into a [`Result`]
    /// # Errors
    /// Passes any errors from the underlying [`KeyValueError`] to the returned `Result`.
    /// # Panics
    /// Panics if the [`KeyValueResult`] is not a [`KeyValueResponse::Set`].
    pub fn unwrap_set(self) -> Result<Option<Vec<u8>>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Set { previous } => Ok(previous.into()),
                _ => {
                    panic!("attempt to convert KeyValueResponse other than Set to Option<Vec<u8>>")
                }
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    /// Converts a [`KeyValueResult`] into a [`Result`]
    /// # Errors
    /// Passes any errors from the underlying [`KeyValueError`] to the returned `Result`.
    /// # Panics
    /// Panics if the [`KeyValueResult`] is not a [`KeyValueResponse::Delete`].
    pub fn unwrap_delete(self) -> Result<Option<Vec<u8>>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Delete { previous } => Ok(previous.into()),
                _ => panic!(
                    "attempt to convert KeyValueResponse other than Delete to Option<Vec<u8>>"
                ),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    /// Converts a [`KeyValueResult`] into a [`Result`]
    /// # Errors
    /// Passes any errors from the underlying [`KeyValueError`] to the returned `Result`.
    /// # Panics
    /// Panics if the [`KeyValueResult`] is not a [`KeyValueResponse::Exists`].
    pub fn unwrap_exists(self) -> Result<bool, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Exists { is_present } => Ok(is_present),
                _ => panic!("attempt to convert KeyValueResponse other than Exists to bool"),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    /// Converts a [`KeyValueResult`] into a [`Result`]
    /// # Errors
    /// Passes any errors from the underlying [`KeyValueError`] to the returned `Result`.
    /// # Panics
    /// Panics if the [`KeyValueResult`] is not a [`KeyValueResponse::ListKeys`].
    pub fn unwrap_list_keys(self) -> Result<(Vec<String>, u64), KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::ListKeys {
                    keys,
                    next_cursor: cursor,
                } => Ok((keys, cursor)),
                _ => panic!(
                    "attempt to convert KeyValueResponse other than ListKeys to (Vec<String>, u64)"
                ),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }
}
