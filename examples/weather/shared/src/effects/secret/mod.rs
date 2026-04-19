//! A custom capability for storing and retrieving secrets (e.g. API keys).
//!
//! The shell-facing protocol is intentionally simple: three operations
//! (fetch, store, delete) with one [`SecretResponse`] enum covering all
//! outcomes. The developer-facing command builders in the [`command`]
//! submodule narrow that wide response into smaller per-operation types
//! ([`SecretFetchResponse`], [`SecretStoreResponse`],
//! [`SecretDeleteResponse`]) so callers only see the variants that apply.

pub mod command;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// The key under which the weather API key is stored.
pub const API_KEY_NAME: &str = "openweather_api_key";

/// Operations the core can ask the shell to perform.
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretRequest {
    /// Fetch the secret stored under the given key (if any).
    Fetch(String),
    /// Store `value` under `key`, replacing any existing value.
    Store(String, String),
    /// Delete the secret stored under the given key.
    Delete(String),
}

impl Operation for SecretRequest {
    type Output = SecretResponse;
}

/// The shell-facing response — every variant any operation might produce.
///
/// The developer-facing command builders narrow this down to the variants
/// a specific operation can actually return.
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretResponse {
    /// Fetch: no secret stored under this key.
    Missing(String),
    /// Fetch: here's the key and its stored value.
    Fetched(String, String),
    /// Store: the secret was stored successfully.
    Stored(String),
    /// Store: storing failed — the string carries the error message.
    StoreError(String),
    /// Delete: the secret was removed.
    Deleted(String),
    /// Delete: deletion failed — the string carries the error message.
    DeleteError(String),
}

/// The developer-facing response for [`command::fetch`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretFetchResponse {
    /// No secret is stored under this key.
    Missing(String),
    /// The stored secret value.
    Fetched(String),
}

/// The developer-facing response for [`command::store`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretStoreResponse {
    /// The secret was stored successfully under `key`.
    Stored(String),
    /// Storage failed; the string carries the error message.
    StoreError(String),
}

/// The developer-facing response for [`command::delete`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretDeleteResponse {
    /// The secret was removed.
    Deleted(String),
    /// Deletion failed; the string carries the error message.
    DeleteError(String),
}
