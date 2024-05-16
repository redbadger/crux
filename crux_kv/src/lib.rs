//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)

pub mod error;

use crux_core::capability::{CapabilityContext, Operation};
use crux_core::macros::Capability;
use error::KeyValueError;
use serde::{Deserialize, Serialize};

/// Supported operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueOperation {
    /// Read bytes stored under a key
    Get { key: String },
    /// Write bytes under a key
    Set { key: String, value: Vec<u8> },
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
    Get { value: Vec<u8> },
    /// Response to a `KeyValueOperation::Set`,
    /// returning the value that was previously stored under the key, may be empty
    Set { previous: Vec<u8> },
    /// Response to a `KeyValueOperation::Delete`,
    /// returning the value that was previously stored under the key, may be empty
    Delete { previous: Vec<u8> },
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
}

#[derive(Capability)]
pub struct KeyValue<Ev> {
    context: CapabilityContext<KeyValueOperation, Ev>,
}

impl<Ev> Clone for KeyValue<Ev> {
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
        }
    }
}

impl<Ev> KeyValue<Ev>
where
    Ev: 'static,
{
    pub fn new(context: CapabilityContext<KeyValueOperation, Ev>) -> Self {
        Self { context }
    }

    /// Read a value under `key`, will dispatch the event with a
    /// `KeyValueResult::Get { value: Vec<u8> }` as payload
    pub fn get<F>(&self, key: String, make_event: F)
    where
        F: FnOnce(Result<Vec<u8>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let this = self.clone();

            async move {
                let response = this.get_async(key).await;
                context.update_app(make_event(response));
            }
        });
    }

    /// Read a value under `key`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn get_async(&self, key: String) -> Result<Vec<u8>, KeyValueError> {
        self.context
            .request_from_shell(KeyValueOperation::Get { key })
            .await
            .unwrap_get()
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    ///
    /// Will dispatch the event with a `KeyValueResult::Set { previous: Vec<u8> }` as payload
    pub fn set<F>(&self, key: String, value: Vec<u8>, make_event: F)
    where
        F: FnOnce(Result<Vec<u8>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let this = self.clone();

            async move {
                let response = this.set_async(key, value).await;
                context.update_app(make_event(response))
            }
        });
    }

    /// Set `key` to be the provided `value`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn set_async(&self, key: String, value: Vec<u8>) -> Result<Vec<u8>, KeyValueError> {
        self.context
            .request_from_shell(KeyValueOperation::Set { key, value })
            .await
            .unwrap_set()
    }

    /// Remove a `key` and its value, will dispatch the event with a
    /// `KeyValueResult::Delete { previous: Vec<u8> }` as payload
    pub fn delete<F>(&self, key: String, make_event: F)
    where
        F: FnOnce(Result<Vec<u8>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        let context = self.context.clone();
        let this = self.clone();

        self.context.spawn(async move {
            let response = this.delete_async(key).await;
            context.update_app(make_event(response))
        });
    }

    /// Remove a `key` and its value, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn delete_async(&self, key: String) -> Result<Vec<u8>, KeyValueError> {
        self.context
            .request_from_shell(KeyValueOperation::Delete { key })
            .await
            .unwrap_delete()
    }

    /// Check to see if a `key` exists, will dispatch the event with a
    /// `KeyValueResult::Exists { is_present: bool }` as payload
    pub fn exists<F>(&self, key: String, make_event: F)
    where
        F: FnOnce(Result<bool, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        let context = self.context.clone();
        let this = self.clone();

        self.context.spawn(async move {
            let response = this.exists_async(key).await;
            context.update_app(make_event(response))
        });
    }

    /// Check to see if a `key` exists, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn exists_async(&self, key: String) -> Result<bool, KeyValueError> {
        self.context
            .request_from_shell(KeyValueOperation::Exists { key })
            .await
            .unwrap_exists()
    }

    /// List keys that start with the provided `prefix`, starting from the provided `cursor`.
    /// Will dispatch the event with a `KeyValueResult::ListKeys { keys: Vec<String>, cursor: u64 }`
    /// as payload.
    ///
    /// A cursor is an opaque value that points to the first key in the next page of keys.
    ///
    /// If the cursor is not found for the specified prefix, the response will include
    /// a `KeyValueError::CursorNotFound` error.
    ///
    /// If the cursor is found the result will be a tuple of the keys and the next cursor
    /// (if there are more keys to list, the cursor will be non-zero, otherwise it will be zero)
    pub fn list_keys<F>(&self, prefix: String, cursor: u64, make_event: F)
    where
        F: FnOnce(Result<(Vec<String>, u64), KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        let context = self.context.clone();
        let this = self.clone();

        self.context.spawn(async move {
            let response = this.list_keys_async(prefix, cursor).await;
            context.update_app(make_event(response))
        });
    }

    /// List keys that start with the provided `prefix`, starting from the provided `cursor`,
    /// while in an async context. This is used together with [`crux_core::compose::Compose`].
    ///
    /// A cursor is an opaque value that points to the first key in the next page of keys.
    ///
    /// If the cursor is not found for the specified prefix, the response will include
    /// a `KeyValueError::CursorNotFound` error.
    ///
    /// If the cursor is found the result will be a tuple of the keys and the next cursor
    /// (if there are more keys to list, the cursor will be non-zero, otherwise it will be zero)
    pub async fn list_keys_async(
        &self,
        prefix: String,
        cursor: u64,
    ) -> Result<(Vec<String>, u64), KeyValueError> {
        self.context
            .request_from_shell(KeyValueOperation::ListKeys { prefix, cursor })
            .await
            .unwrap_list_keys()
    }
}

impl KeyValueResult {
    fn unwrap_get(self) -> Result<Vec<u8>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Get { value } => Ok(value),
                _ => panic!("attempt to convert KeyValueResponse other than Get to Vec<u8>"),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    fn unwrap_set(self) -> Result<Vec<u8>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Set { previous } => Ok(previous),
                _ => panic!("attempt to convert KeyValueResponse other than Set to Vec<u8>"),
            },
            KeyValueResult::Err { error } => Err(error.clone()),
        }
    }

    fn unwrap_delete(self) -> Result<Vec<u8>, KeyValueError> {
        match self {
            KeyValueResult::Ok { response } => match response {
                KeyValueResponse::Delete { previous } => Ok(previous),
                _ => panic!("attempt to convert KeyValueResponse other than Delete to Vec<u8>"),
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

#[cfg(test)]
mod tests;
