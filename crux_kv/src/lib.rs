//! A basic Key-Value store for use with Crux
//!
//! `crux_kv` allows Crux apps to store and retrieve arbitrary data by asking the Shell to
//! persist the data using platform native capabilities (e.g. disk or web localStorage)
//!
//! This is still work in progress and extremely basic.
use crux_core::capability::{CapabilityContext, Operation};
use crux_core::macros::Capability;
use serde::{Deserialize, Serialize};

/// Supported operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueOperation {
    /// Read bytes stored under a key
    Read(String),
    /// Write bytes under a key
    Write(String, Vec<u8>),
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum KeyValueOutput {
    // TODO: Add support for errors
    Read(Option<Vec<u8>>),
    // TODO: Add support for errors
    Write(bool),
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
    /// `KeyValueOutput::Read(Option<Vec<u8>>)` as payload
    pub fn read<F>(&self, key: &str, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        let ctx = self.context.clone();
        let key = key.to_string();
        self.context.spawn(async move {
            let output = ctx.request_from_shell(KeyValueOperation::Read(key)).await;

            ctx.update_app(make_event(output))
        });
    }

    /// Read a value under `key`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn read_async(&self, key: &str) -> KeyValueOutput {
        self.context
            .request_from_shell(KeyValueOperation::Read(key.to_string()))
            .await
    }

    /// Set `key` to be the provided `value`. Typically the bytes would be
    /// a value serialized/deserialized by the app.
    ///
    /// Will dispatch the event with a `KeyValueOutput::Write(bool)` as payload
    pub fn write<F>(&self, key: &str, value: Vec<u8>, make_event: F)
    where
        F: Fn(KeyValueOutput) -> Ev + Send + Sync + 'static,
    {
        self.context.spawn({
            let context = self.context.clone();
            let key = key.to_string();
            async move {
                let resp = context
                    .request_from_shell(KeyValueOperation::Write(key, value))
                    .await;

                context.update_app(make_event(resp))
            }
        });
    }

    /// Set `key` to be the provided `value`, while in an async context. This is used together with
    /// [`crux_core::compose::Compose`].
    pub async fn write_async(&self, key: &str, value: Vec<u8>) -> KeyValueOutput {
        self.context
            .request_from_shell(KeyValueOperation::Write(key.to_string(), value))
            .await
    }
}
