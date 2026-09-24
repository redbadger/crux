//! One type per store operation.
//!
//! Each type in this module is a single operation the shell can perform, with
//! exactly one output type and a statically declared
//! [`OperationKind`](crux_core::OperationKind). That is the difference from
//! [`KeyValueOperation`](crate::KeyValueOperation), where every variant shares
//! one [`KeyValueResult`](crate::KeyValueResult) and the capability has to
//! check at runtime that the shell answered the question it was asked.
//!
//! ```
//! # use crux_core::{Command, macros::effect, render::RenderOperation};
//! use crux_kv::{operation, store::KeyValue};
//!
//! #[effect]
//! enum Effect {
//!     KvGet(operation::GetValue),
//!     KvSet(operation::SetValue),
//!     Render(RenderOperation),
//! }
//!
//! # enum Event { Loaded(crux_kv::DataResult) }
//! let command: Command<Effect, Event> =
//!     KeyValue::get("key").then_send(Event::Loaded);
//! ```
//!
//! The outputs are the wire types the shell resolves with:
//! [`ValueResult`], [`ExistsResult`] and [`KeysResult`]. Each converts to and
//! from the [`Result`] alias an app sees — [`DataResult`], [`StatusResult`]
//! and [`ListResult`] — so a shell written against either API can serve the
//! other.

// `#[derive(Facet)]` generates `unsafe` methods.
#![allow(clippy::unsafe_derive_deserialize)]

use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

use crate::{
    error::{DataResult, KeyValueError, ListResult, StatusResult},
    protocol::Value,
};

/// Read the bytes stored under `key`.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = ValueResult)]
pub struct GetValue {
    pub key: String,
}

/// Write `value` under `key`, answering with the value it replaced.
#[derive(Operation, Facet, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = ValueResult)]
pub struct SetValue {
    pub key: String,
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,
}

/// Remove `key` and its value, answering with the value it removed.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = ValueResult)]
pub struct DeleteValue {
    pub key: String,
}

/// Test whether `key` is present in the store.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = ExistsResult)]
pub struct KeyExists {
    pub key: String,
}

/// List the keys that start with `prefix`, starting at `cursor`.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = KeysResult)]
pub struct ListKeys {
    /// The prefix to list keys for, or an empty string to list all keys
    pub prefix: String,
    /// The cursor to start listing from, or 0 to start from the beginning.
    /// If there are more keys to list, the response will include a new cursor.
    /// If there are no more keys, the response will include a cursor of 0.
    /// The cursor is opaque to the caller, and should be passed back to the
    /// [`ListKeys`] operation to continue listing keys.
    /// If the cursor is not found for the specified prefix, the response will
    /// include a [`KeyValueError::CursorNotFound`] error.
    pub cursor: u64,
}

/// The value a [`GetValue`], [`SetValue`] or [`DeleteValue`] answers with, or
/// the error that prevented it.
///
/// Note: we can't use [`core::result::Result`] here because it is not
/// currently supported across the FFI boundary, when using `facet_typegen`.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum ValueResult {
    Ok(Value),
    Err(KeyValueError),
}

/// The answer to a [`KeyExists`], or the error that prevented it.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum ExistsResult {
    Ok(bool),
    Err(KeyValueError),
}

/// The answer to a [`ListKeys`], or the error that prevented it.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum KeysResult {
    Ok(KeyPage),
    Err(KeyValueError),
}

/// A page of keys, and the cursor to continue from.
///
/// Named `KeyPage` rather than `Keys` because a generated C# class cannot have
/// a member with its own name (CS0542), and this one's first field is `keys`.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct KeyPage {
    pub keys: Vec<String>,
    /// The cursor to continue listing keys, or 0 if there are no more keys.
    pub next_cursor: u64,
}

impl std::fmt::Debug for SetValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value_repr = std::str::from_utf8(&self.value).map_or_else(
            |_| format!("<binary data - {} bytes>", self.value.len()),
            |s| {
                if s.len() < 50 {
                    format!("\"{s}\"")
                } else {
                    format!("\"{}\"...", s.chars().take(50).collect::<String>())
                }
            },
        );

        f.debug_struct("SetValue")
            .field("key", &self.key)
            .field("value", &format_args!("{value_repr}"))
            .finish()
    }
}

impl From<ValueResult> for DataResult {
    fn from(result: ValueResult) -> Self {
        match result {
            ValueResult::Ok(value) => Ok(value.into()),
            ValueResult::Err(error) => Err(error),
        }
    }
}

impl From<DataResult> for ValueResult {
    fn from(result: DataResult) -> Self {
        match result {
            Ok(value) => Self::Ok(value.into()),
            Err(error) => Self::Err(error),
        }
    }
}

impl From<ExistsResult> for StatusResult {
    fn from(result: ExistsResult) -> Self {
        match result {
            ExistsResult::Ok(is_present) => Ok(is_present),
            ExistsResult::Err(error) => Err(error),
        }
    }
}

impl From<StatusResult> for ExistsResult {
    fn from(result: StatusResult) -> Self {
        match result {
            Ok(is_present) => Self::Ok(is_present),
            Err(error) => Self::Err(error),
        }
    }
}

impl From<KeysResult> for ListResult {
    fn from(result: KeysResult) -> Self {
        match result {
            KeysResult::Ok(KeyPage { keys, next_cursor }) => Ok((keys, next_cursor)),
            KeysResult::Err(error) => Err(error),
        }
    }
}

impl From<ListResult> for KeysResult {
    fn from(result: ListResult) -> Self {
        match result {
            Ok((keys, next_cursor)) => Self::Ok(KeyPage { keys, next_cursor }),
            Err(error) => Self::Err(error),
        }
    }
}
