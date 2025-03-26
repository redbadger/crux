//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)

pub mod command;
pub mod error;
pub mod value;

use serde::{Deserialize, Serialize};

use crux_core::capability::{CapabilityContext, Operation};

use error::KeyValueError;
use value::Value;

/// Supported operations
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
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
                    .field("value", &format_args!("{}", body_repr))
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
    fn register_types(generator: &mut crux_core::typegen::TypeGen) -> crux_core::typegen::Result {
        generator.register_type::<KeyValueResponse>()?;
        generator.register_type::<KeyValueError>()?;
        generator.register_type::<Value>()?;
        generator.register_type::<Self>()?;
        generator.register_type::<Self::Output>()?;
        Ok(())
    }
}

pub struct KeyValue<Ev> {
    context: CapabilityContext<KeyValueOperation, Ev>,
}

impl<Ev> crux_core::Capability<Ev> for KeyValue<Ev> {
    type Operation = KeyValueOperation;

    type MappedSelf<MappedEv> = KeyValue<MappedEv>;

    fn map_event<F, NewEv>(&self, f: F) -> Self::MappedSelf<NewEv>
    where
        F: Fn(NewEv) -> Ev + Send + Sync + 'static,
        Ev: 'static,
        NewEv: 'static + Send,
    {
        KeyValue::new(self.context.map_event(f))
    }
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
        F: FnOnce(Result<Option<Vec<u8>>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = get(&context, key).await;
                context.update_app(make_event(response));
            }
        });
    }

    /// Read a value under `key`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    ///
    /// Returns the value stored under the key, or `None` if the key is not present.
    pub async fn get_async(&self, key: String) -> Result<Option<Vec<u8>>, KeyValueError> {
        get(&self.context, key).await
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    ///
    /// Will dispatch the event with a `KeyValueResult::Set { previous: Vec<u8> }` as payload
    pub fn set<F>(&self, key: String, value: Vec<u8>, make_event: F)
    where
        F: FnOnce(Result<Option<Vec<u8>>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = set(&context, key, value).await;
                context.update_app(make_event(response))
            }
        });
    }

    /// Set `key` to be the provided `value`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    ///
    /// Returns the previous value stored under the key, if any.
    pub async fn set_async(
        &self,
        key: String,
        value: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, KeyValueError> {
        set(&self.context, key, value).await
    }

    /// Remove a `key` and its value, will dispatch the event with a
    /// `KeyValueResult::Delete { previous: Vec<u8> }` as payload
    pub fn delete<F>(&self, key: String, make_event: F)
    where
        F: FnOnce(Result<Option<Vec<u8>>, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = delete(&context, key).await;
                context.update_app(make_event(response))
            }
        });
    }

    /// Remove a `key` and its value, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    ///
    /// Returns the previous value stored under the key, if any.
    pub async fn delete_async(&self, key: String) -> Result<Option<Vec<u8>>, KeyValueError> {
        delete(&self.context, key).await
    }

    /// Check to see if a `key` exists, will dispatch the event with a
    /// `KeyValueResult::Exists { is_present: bool }` as payload
    pub fn exists<F>(&self, key: String, make_event: F)
    where
        F: FnOnce(Result<bool, KeyValueError>) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = exists(&context, key).await;
                context.update_app(make_event(response))
            }
        });
    }

    /// Check to see if a `key` exists, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    ///
    /// Returns `true` if the key exists, `false` otherwise.
    pub async fn exists_async(&self, key: String) -> Result<bool, KeyValueError> {
        exists(&self.context, key).await
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
        self.context.spawn({
            let context = self.context.clone();
            async move {
                let response = list_keys(&context, prefix, cursor).await;
                context.update_app(make_event(response))
            }
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
        list_keys(&self.context, prefix, cursor).await
    }
}

async fn get<Ev: 'static>(
    context: &CapabilityContext<KeyValueOperation, Ev>,
    key: String,
) -> Result<Option<Vec<u8>>, KeyValueError> {
    context
        .request_from_shell(KeyValueOperation::Get { key })
        .await
        .unwrap_get()
}

async fn set<Ev: 'static>(
    context: &CapabilityContext<KeyValueOperation, Ev>,
    key: String,
    value: Vec<u8>,
) -> Result<Option<Vec<u8>>, KeyValueError> {
    context
        .request_from_shell(KeyValueOperation::Set { key, value })
        .await
        .unwrap_set()
}

async fn delete<Ev: 'static>(
    context: &CapabilityContext<KeyValueOperation, Ev>,
    key: String,
) -> Result<Option<Vec<u8>>, KeyValueError> {
    context
        .request_from_shell(KeyValueOperation::Delete { key })
        .await
        .unwrap_delete()
}

async fn exists<Ev: 'static>(
    context: &CapabilityContext<KeyValueOperation, Ev>,
    key: String,
) -> Result<bool, KeyValueError> {
    context
        .request_from_shell(KeyValueOperation::Exists { key })
        .await
        .unwrap_exists()
}

async fn list_keys<Ev: 'static>(
    context: &CapabilityContext<KeyValueOperation, Ev>,
    prefix: String,
    cursor: u64,
) -> Result<(Vec<String>, u64), KeyValueError> {
    context
        .request_from_shell(KeyValueOperation::ListKeys { prefix, cursor })
        .await
        .unwrap_list_keys()
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

#[cfg(test)]
mod tests;
