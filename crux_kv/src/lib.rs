#![deny(clippy::pedantic)]
//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)

pub mod command;
pub mod error;
pub mod value;

use facet::Facet;
use serde::{Deserialize, Serialize};

use crux_core::{Command, Request, capability::Operation, command::RequestBuilder};

use error::KeyValueError;
use value::Value;

use std::{future::Future, marker::PhantomData};

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
/// Note: we can't use `Result` and `Option` here because generics are not currently
/// supported across the FFI boundary, when using the builtin typegen.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueResult {
    Ok { response: KeyValueResponse },
    Err { error: KeyValueError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    fn unwrap_get(self) -> Result<Option<Vec<u8>>, KeyValueError> {
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

    fn unwrap_set(self) -> Result<Option<Vec<u8>>, KeyValueError> {
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

    fn unwrap_delete(self) -> Result<Option<Vec<u8>>, KeyValueError> {
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

    fn unwrap_exists(self) -> Result<bool, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Exists { is_present } => Ok(is_present),
                _ => panic!("attempt to convert KeyValueResponse other than Exists to bool"),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    fn unwrap_list_keys(self) -> Result<(Vec<String>, u64), KeyValueError> {
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

pub struct KeyValue<Effect, Event> {
    // Allow the impl to declare trait bounds once. Thanks rustc
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

type StatusResult = Result<bool, KeyValueError>;
type DataResult = Result<Option<Vec<u8>>, KeyValueError>;
type ListResult = Result<(Vec<String>, u64), KeyValueError>;

impl<Effect, Event> KeyValue<Effect, Event>
where
    Effect: Send + From<Request<KeyValueOperation>> + 'static,
    Event: Send + 'static,
{
    /// Read a value under `key`
    pub fn get(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Get { key: key.into() })
            .map(KeyValueResult::unwrap_get)
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    pub fn set(
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Set {
            key: key.into(),
            value,
        })
        .map(KeyValueResult::unwrap_set)
    }

    /// Remove a `key` and its value, return previous value if it existed
    pub fn delete(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Delete { key: key.into() })
            .map(KeyValueResult::unwrap_delete)
    }

    /// Check to see if a `key` exists
    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = StatusResult>> {
        Command::request_from_shell(KeyValueOperation::Exists { key: key.into() })
            .map(KeyValueResult::unwrap_exists)
    }

    /// List keys that start with the provided `prefix`, starting from the provided `cursor`.
    ///
    /// A cursor is an opaque value that points to the first key in the next page of keys.
    ///
    /// If the cursor is not found for the specified prefix, the response will include
    /// a `KeyValueError::CursorNotFound` error.
    ///
    /// If the cursor is found the result will be a tuple of the keys and the next cursor
    /// (if there are more keys to list, the cursor will be non-zero, otherwise it will be zero)
    pub fn list_keys(
        prefix: impl Into<String>,
        cursor: u64,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = ListResult>> {
        Command::request_from_shell(KeyValueOperation::ListKeys {
            prefix: prefix.into(),
            cursor,
        })
        .map(KeyValueResult::unwrap_list_keys)
    }
}

#[cfg(test)]
mod tests;
