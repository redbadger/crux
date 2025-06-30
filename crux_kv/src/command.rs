//! The Command based API for `crux_kv`

use std::{future::Future, marker::PhantomData};

use crux_core::{Command, Request, command::RequestBuilder};

use crate::{KeyValueOperation, error::KeyValueError};

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
            .map(super::KeyValueResult::unwrap_get)
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
        .map(super::KeyValueResult::unwrap_set)
    }

    /// Remove a `key` and its value, return previous value if it existed
    pub fn delete(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Delete { key: key.into() })
            .map(super::KeyValueResult::unwrap_delete)
    }

    /// Check to see if a `key` exists
    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = StatusResult>> {
        Command::request_from_shell(KeyValueOperation::Exists { key: key.into() })
            .map(super::KeyValueResult::unwrap_exists)
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
        .map(super::KeyValueResult::unwrap_list_keys)
    }
}
