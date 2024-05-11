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
pub enum KeyValueRequest {
    /// Read bytes stored under a key
    Get { key: String },
    /// Write bytes under a key
    Set { key: String, value: Vec<u8> },
    /// Remove a key and its value
    Delete { key: String },
    /// Test if a key exists
    Exists { key: String },
}

/// The result of reading from the key value store.
///
/// Note: we can't use `Result` and `Option` here because generics are not currently
/// supported across the FFI boundary, when using the builtin typegen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyValueReadResult {
    /// the value stored under the key, may be empty
    Data { value: Vec<u8> },
    /// whether the key exists or not
    Exists { value: bool },
    /// an error occurred
    Err { error: KeyValueError },
}

/// The result of writing to the key value store.
///
/// Note: we can't use `Result` and `Option` here because generics are not currently
/// supported across the FFI boundary, when using the builtin typegen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyValueWriteResult {
    /// the previous value stored under the key, may be empty
    Ok { previous: Vec<u8> },
    /// an error occurred
    Err { error: KeyValueError },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueResponse {
    Exists { result: KeyValueReadResult },
    Get { result: KeyValueReadResult },
    Set { result: KeyValueWriteResult },
    Delete { result: KeyValueWriteResult },
}

impl Operation for KeyValueRequest {
    type Output = KeyValueResponse;
}

#[derive(Capability)]
pub struct KeyValue<Ev> {
    context: CapabilityContext<KeyValueRequest, Ev>,
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
    pub fn new(context: CapabilityContext<KeyValueRequest, Ev>) -> Self {
        Self { context }
    }

    /// Read a value under `key`, will dispatch the event with a
    /// `KeyValueOutput::Get(KeyValueResult<Option<Vec<u8>>>)` as payload
    pub fn get<F>(&self, key: String, make_event: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
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
    pub async fn get_async(&self, key: String) -> KeyValueResponse {
        let response = self
            .context
            .request_from_shell(KeyValueRequest::Get { key })
            .await;

        assert!(
            matches!(response, KeyValueResponse::Get { .. }),
            "unexpected response: {:?}",
            response
        );

        response
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    ///
    /// Will dispatch the event with a `KeyValueOutput::Set { result: KeyValueResult<()> }` as payload
    pub fn set<F>(&self, key: String, value: Vec<u8>, make_event: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
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
    pub async fn set_async(&self, key: String, value: Vec<u8>) -> KeyValueResponse {
        let response = self
            .context
            .request_from_shell(KeyValueRequest::Set { key, value })
            .await;

        assert!(
            matches!(response, KeyValueResponse::Set { .. }),
            "unexpected response: {:?}",
            response
        );

        response
    }

    /// Remove a `key` and its value, will dispatch the event with a
    /// `KeyValueOutput::Delete(KeyValueResult<()>)` as payload
    pub fn delete<F>(&self, key: String, make_event: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
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
    pub async fn delete_async(&self, key: String) -> KeyValueResponse {
        let response = self
            .context
            .request_from_shell(KeyValueRequest::Delete { key })
            .await;

        assert!(
            matches!(response, KeyValueResponse::Delete { .. }),
            "unexpected response: {:?}",
            response
        );

        response
    }

    /// Check to see if a `key` exists, will dispatch the event with a
    /// `KeyValueOutput::Exists(KeyValueResult<bool>)` as payload
    pub fn exists<F>(&self, key: String, make_event: F)
    where
        F: Fn(KeyValueResponse) -> Ev + Send + Sync + 'static,
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
    pub async fn exists_async(&self, key: String) -> KeyValueResponse {
        let response = self
            .context
            .request_from_shell(KeyValueRequest::Exists { key })
            .await;

        assert!(
            matches!(response, KeyValueResponse::Exists { .. }),
            "unexpected response: {:?}",
            response
        );

        response
    }
}

#[cfg(test)]
mod tests;
