//! The Command based API for crux_kv

use std::{future::Future, marker::PhantomData};

use crux_core::{command::RequestBuilder, Command, Request};

use super::{KeyValueOperation, KeyValueResult};

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
    pub fn get(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = KeyValueResult>> {
        Command::request_from_shell(KeyValueOperation::Get { key: key.into() })
    }

    pub fn set(
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = KeyValueResult>> {
        Command::request_from_shell(KeyValueOperation::Set {
            key: key.into(),
            value,
        })
    }

    pub fn delete(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = KeyValueResult>> {
        Command::request_from_shell(KeyValueOperation::Delete { key: key.into() })
    }

    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = KeyValueResult>> {
        Command::request_from_shell(KeyValueOperation::Exists { key: key.into() })
    }

    pub fn list_keys(
        prefix: impl Into<String>,
        cursor: u64,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = KeyValueResult>> {
        Command::request_from_shell(KeyValueOperation::ListKeys {
            prefix: prefix.into(),
            cursor,
        })
    }
}
