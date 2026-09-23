//! The key-value store capability, one operation type at a time.
//!
//! This module holds [`KeyValue`], the per-operation key-value capability. The
//! type of the same name at the crate root — [`crate::KeyValue`] — is the
//! enum-based API that this one replaces.
//!
//! The two coexist in this release, so an app can move one call at a time
//! rather than rewriting every store interaction at once. A later breaking
//! release removes the root type and re-exports this one in its place:
//! `crux_kv::store::KeyValue` will go on meaning exactly what it means today,
//! and `crux_kv::KeyValue` will come to mean this type.

use std::{future::Future, marker::PhantomData};

use crux_core::{Command, Request, command::RequestBuilder};

use crate::{
    error::{DataResult, ListResult, StatusResult},
    operation,
};

/// A key-value store, one operation type at a time.
///
/// The same API as the enum-based [`KeyValue`](crate::KeyValue) at the crate
/// root, built from the per-operation types in [`operation`]: each method
/// sends its own operation type, whose single output type the shell cannot get
/// wrong.
///
/// The bounds are per method, so an app's `Effect` only has to carry the
/// operations it actually uses:
///
/// ```
/// # use crux_core::{Command, macros::effect};
/// use crux_kv::{operation, store::KeyValue};
///
/// #[effect]
/// enum Effect {
///     KvGet(operation::GetValue),
///     KvSet(operation::SetValue),
/// }
///
/// # enum Event { Loaded(crux_kv::DataResult), Saved(crux_kv::DataResult) }
/// let load: Command<Effect, Event> = KeyValue::get("key").then_send(Event::Loaded);
/// let save: Command<Effect, Event> =
///     KeyValue::set("key", b"value".to_vec()).then_send(Event::Saved);
/// ```
pub struct KeyValue<Effect, Event> {
    // Allow the impl to declare trait bounds once. Thanks rustc
    effect: PhantomData<Effect>,
    event: PhantomData<Event>,
}

impl<Effect, Event> KeyValue<Effect, Event>
where
    Effect: Send + 'static,
    Event: Send + 'static,
{
    /// Read a value under `key`
    pub fn get(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>>
    where
        Effect: From<Request<operation::GetValue>>,
    {
        Command::request_from_shell(operation::GetValue { key: key.into() }).map(Into::into)
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    pub fn set(
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>>
    where
        Effect: From<Request<operation::SetValue>>,
    {
        Command::request_from_shell(operation::SetValue {
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
        Effect: From<Request<operation::DeleteValue>>,
    {
        Command::request_from_shell(operation::DeleteValue { key: key.into() }).map(Into::into)
    }

    /// Check to see if a `key` exists
    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = StatusResult>>
    where
        Effect: From<Request<operation::KeyExists>>,
    {
        Command::request_from_shell(operation::KeyExists { key: key.into() }).map(Into::into)
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
