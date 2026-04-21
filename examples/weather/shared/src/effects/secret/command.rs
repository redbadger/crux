//! Command builders for the [secret capability](super).
//!
//! Each builder issues one [`SecretRequest`] and narrows the shell's wide
//! [`SecretResponse`] down to the [`SecretFetchResponse`],
//! [`SecretStoreResponse`], or [`SecretDeleteResponse`] that's relevant
//! to that operation. They're generic over `Effect` and `Event` so any
//! Crux app can adopt them.

use std::future::Future;

use crux_core::Request;
use crux_core::command::RequestBuilder;

use super::{
    SecretDeleteResponse, SecretFetchResponse, SecretRequest, SecretResponse, SecretStoreResponse,
};

/// Fetches the secret stored under `key`, if any.
#[must_use]
pub fn fetch<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretFetchResponse>>
where
    Ef: From<Request<SecretRequest>> + Send + 'static,
    Ev: Send + 'static,
{
    let key = key.into();
    crux_core::Command::request_from_shell(SecretRequest::Fetch(key)).map(|response| match response
    {
        SecretResponse::Missing(key) => SecretFetchResponse::Missing(key),
        SecretResponse::Fetched(_, value) => SecretFetchResponse::Fetched(value),
        _ => unreachable!("fetch only produces Missing or Fetched"),
    })
}

/// Stores `value` under `key`, replacing any existing secret.
#[must_use]
pub fn store<Ef, Ev>(
    key: impl Into<String>,
    value: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretStoreResponse>>
where
    Ef: From<Request<SecretRequest>> + Send + 'static,
    Ev: Send + 'static,
{
    let key = key.into();
    let value = value.into();
    crux_core::Command::request_from_shell(SecretRequest::Store(key, value)).map(|response| {
        match response {
            SecretResponse::Stored(key) => SecretStoreResponse::Stored(key),
            SecretResponse::StoreError(msg) => SecretStoreResponse::StoreError(msg),
            _ => unreachable!("store only produces Stored or StoreError"),
        }
    })
}

/// Deletes the secret stored under `key`.
#[must_use]
pub fn delete<Ef, Ev>(
    key: impl Into<String>,
) -> RequestBuilder<Ef, Ev, impl Future<Output = SecretDeleteResponse>>
where
    Ef: From<Request<SecretRequest>> + Send + 'static,
    Ev: Send + 'static,
{
    let key = key.into();
    crux_core::Command::request_from_shell(SecretRequest::Delete(key)).map(
        |response| match response {
            SecretResponse::Deleted(key) => SecretDeleteResponse::Deleted(key),
            SecretResponse::DeleteError(msg) => SecretDeleteResponse::DeleteError(msg),
            _ => unreachable!("delete only produces Deleted or DeleteError"),
        },
    )
}
