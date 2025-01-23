//! The Command based API for crux_kv

use std::{future::Future, marker::PhantomData};

use crux_core::{command::RequestBuilder, Command, Request};

use crate::{error::KeyValueError, KeyValueOperation};

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
    pub fn get(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Get { key: key.into() })
            .map(|kv_result| kv_result.unwrap_get())
    }

    pub fn set(
        key: impl Into<String>,
        value: Vec<u8>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Set {
            key: key.into(),
            value,
        })
        .map(|kv_result| kv_result.unwrap_set())
    }

    pub fn delete(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = DataResult>> {
        Command::request_from_shell(KeyValueOperation::Delete { key: key.into() })
            .map(|kv_result| kv_result.unwrap_delete())
    }

    pub fn exists(
        key: impl Into<String>,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = StatusResult>> {
        Command::request_from_shell(KeyValueOperation::Exists { key: key.into() })
            .map(|kv_result| kv_result.unwrap_exists())
    }

    pub fn list_keys(
        prefix: impl Into<String>,
        cursor: u64,
    ) -> RequestBuilder<Effect, Event, impl Future<Output = ListResult>> {
        Command::request_from_shell(KeyValueOperation::ListKeys {
            prefix: prefix.into(),
            cursor,
        })
        .map(|kv_result| kv_result.unwrap_list_keys())
    }
}
