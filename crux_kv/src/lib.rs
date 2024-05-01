//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)

pub mod error;

use crux_core::capability::{CapabilityContext, Operation};
use crux_core::macros::Capability;
use error::KeyValueResult;
use serde::{Deserialize, Serialize};

/// Supported operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeyValueOperation {
    /// Read bytes stored under a key
    Get { key: String },
    /// Write bytes under a key
    Set { key: String, value: Vec<u8> },
    /// Remove a key and its value
    Delete { key: String },
    /// Test if a key exists
    Exists { key: String },
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum KeyValueOutput {
    Get {
        value: KeyValueResult<Option<Vec<u8>>>,
    },
    Set {
        result: KeyValueResult<()>,
    },
    Delete {
        result: KeyValueResult<()>,
    },
    Exists {
        result: KeyValueResult<bool>,
    },
}

impl Operation for KeyValueOperation {
    type Output = KeyValueOutput;
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
    /// `KeyValueOutput::Get(KeyValueResult<Option<Vec<u8>>>)` as payload
    pub fn get<F>(&self, key: &str, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let output = ctx.request_from_shell(KeyValueOperation::Get { key }).await;

            ctx.update_app(make_event(output))
        });
    }

    /// Read a value under `key`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn get_async(&self, key: &str) -> KeyValueOutput {
        let key = key.to_string();
        self.context
            .request_from_shell(KeyValueOperation::Get { key })
            .await
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    ///
    /// Will dispatch the event with a `KeyValueOutput::Set { result: KeyValueResult<()> }` as payload
    pub fn set<F>(&self, key: &str, value: Vec<u8>, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let key = key.to_string();
            async move {
                let resp = context
                    .request_from_shell(KeyValueOperation::Set { key, value })
                    .await;

                context.update_app(make_event(resp))
            }
        });
    }

    /// Set `key` to be the provided `value`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn set_async(&self, key: &str, value: Vec<u8>) -> KeyValueOutput {
        let key = key.to_string();
        self.context
            .request_from_shell(KeyValueOperation::Set { key, value })
            .await
    }

    /// Remove a `key` and its value, will dispatch the event with a
    /// `KeyValueOutput::Delete(KeyValueResult<()>)` as payload
    pub fn delete<F>(&self, key: &str, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let output = ctx
                .request_from_shell(KeyValueOperation::Delete { key })
                .await;

            ctx.update_app(make_event(output))
        });
    }

    /// Remove a `key` and its value, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn delete_async(&self, key: &str) -> KeyValueOutput {
        let key = key.to_string();
        self.context
            .request_from_shell(KeyValueOperation::Delete { key })
            .await
    }

    /// Check to see if a `key` exists, will dispatch the event with a
    /// `KeyValueOutput::Exists(KeyValueResult<bool>)` as payload
    pub fn exists<F>(&self, key: &str, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let output = ctx
                .request_from_shell(KeyValueOperation::Exists { key })
                .await;

            ctx.update_app(make_event(output))
        });
    }

    /// Check to see if a `key` exists, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn exists_async(&self, key: &str) -> KeyValueOutput {
        let key = key.to_string();
        self.context
            .request_from_shell(KeyValueOperation::Exists { key })
            .await
    }
}

#[cfg(test)]
mod tests;
