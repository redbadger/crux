//! Command builders for the [secret capability](super).
//!
//! Each builder issues one operation and hands back that operation's output
//! unchanged — [`SecretFetchResponse`], [`SecretStoreResponse`] or
//! [`SecretDeleteResponse`]. They're generic over `Effect` and `Event` so any
//! Crux app can adopt them.

use std::future::Future;

use crux_core::Request;
use crux_core::command::RequestBuilder;

use super::{Delete, Fetch, SecretDeleteResponse, SecretFetchResponse, SecretStoreResponse, Store};

/// Fetches the secret stored under `key`, if any.
#[must_use]
pub fn fetch<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretFetchResponse>>
where
    Ef: From<Request<Fetch>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(Fetch(key.into()))
}

/// Stores `value` under `key`, replacing any existing secret.
#[must_use]
pub fn store<Ef, Ev>(
    key: impl Into<String>,
    value: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretStoreResponse>>
where
    Ef: From<Request<Store>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(Store(key.into(), value.into()))
}

/// Deletes the secret stored under `key`.
#[must_use]
pub fn delete<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretDeleteResponse>>
where
    Ef: From<Request<Delete>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(Delete(key.into()))
}
