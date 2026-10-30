//! Command builders for the [secret capability](super).
//!
//! Each builder issues one operation and hands back that operation's output
//! unchanged — [`SecretFetchResponse`], [`SecretStoreResponse`] or
//! [`SecretDeleteResponse`]. They're generic over `Effect` and `Event` so any
//! Crux app can adopt them.

use std::future::Future;

use crux_core::Request;
use crux_core::command::RequestBuilder;

use super::{
    DeleteSecret, FetchSecret, SecretDeleteResponse, SecretFetchResponse, SecretStoreResponse,
    StoreSecret,
};

/// Fetches the secret stored under `key`, if any.
#[must_use]
pub fn fetch<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretFetchResponse>>
where
    Ef: From<Request<FetchSecret>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(FetchSecret(key.into()))
}

/// Stores `value` under `key`, replacing any existing secret.
#[must_use]
pub fn store<Ef, Ev>(
    key: impl Into<String>,
    value: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretStoreResponse>>
where
    Ef: From<Request<StoreSecret>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(StoreSecret(key.into(), value.into()))
}

/// Deletes the secret stored under `key`.
#[must_use]
pub fn delete<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretDeleteResponse>>
where
    Ef: From<Request<DeleteSecret>> + Send + 'static,
    Ev: Send + 'static,
{
    crux_core::Command::request_from_shell(DeleteSecret(key.into()))
}
