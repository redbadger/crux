//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)

pub mod command;
pub mod error;
pub mod operation;
pub mod protocol;

use std::{future::Future, marker::PhantomData};

use crux_core::{Command, Request, command::RequestBuilder};

pub use error::*;
pub use protocol::*;

pub struct KeyValue<Effect, Event> {
    // Allow the impl to declare trait bounds once. Thanks rustc
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

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

/// A key-value store, one operation type at a time.
///
/// The same API as [`KeyValue`], built from the per-operation types in
/// [`operation`]: each method sends its own operation type, whose single
/// output type the shell cannot get wrong.
///
/// The bounds are per method, so an app's `Effect` only has to carry the
/// operations it actually uses:
///
/// ```
/// # use crux_core::{Command, macros::effect};
/// use crux_kv::{KeyValueStore, operation};
///
/// #[effect]
/// enum Effect {
///     Get(operation::Get),
///     Set(operation::Set),
/// }
///
/// # enum Event { Loaded(crux_kv::DataResult), Saved(crux_kv::DataResult) }
/// let load: Command<Effect, Event> = KeyValueStore::get("key").then_send(Event::Loaded);
/// let save: Command<Effect, Event> =
///     KeyValueStore::set("key", b"value".to_vec()).then_send(Event::Saved);
/// ```
pub struct KeyValueStore<Effect, Event> {
    // Allow the impl to declare trait bounds once. Thanks rustc
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> KeyValueStore<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    /// Read a value under `key`
    pub fn get(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>>
    where
        Effect: From<Request<operation::Get>>,
    {
        Command::request_from_shell(operation::Get { key: key.into() }).map(Into::into)
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    pub fn set(
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>>
    where
        Effect: From<Request<operation::Set>>,
    {
        Command::request_from_shell(operation::Set {
            key: key.into(),
            value,
        })
        .map(Into::into)
    }

    /// Remove a `key` and its value, return previous value if it existed
    pub fn delete(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>>
    where
        Effect: From<Request<operation::Delete>>,
    {
        Command::request_from_shell(operation::Delete { key: key.into() }).map(Into::into)
    }

    /// Check to see if a `key` exists
    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = StatusResult>>
    where
        Effect: From<Request<operation::Exists>>,
    {
        Command::request_from_shell(operation::Exists { key: key.into() }).map(Into::into)
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
    ) -> RequestBuilder<Effect, Event, impl Future<Output = ListResult>>
    where
        Effect: From<Request<operation::ListKeys>>,
    {
        Command::request_from_shell(operation::ListKeys {
            prefix: prefix.into(),
            cursor,
        })
        .map(Into::into)
    }
}

#[cfg(test)]
mod tests;
