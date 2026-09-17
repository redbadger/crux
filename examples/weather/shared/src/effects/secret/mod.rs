//! A custom capability for storing and retrieving secrets (e.g. API keys).
//!
//! Three operations — [`FetchSecret`], [`StoreSecret`] and [`DeleteSecret`] —
//! each with its own output type naming only the outcomes that operation can
//! actually have: [`SecretFetchResponse`], [`SecretStoreResponse`] and
//! [`SecretDeleteResponse`]. There is no wide response enum shared between
//! them, so no call site has to rule out variants that cannot happen. The
//! developer-facing command builders live in the [`command`] submodule.
//!
//! The names carry the capability because the generated shell module is one
//! flat namespace: a bare `Delete` here and `crux_kv`'s `Delete` would both
//! generate as `Delete`, and the app registers both capabilities.

pub mod command;

use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// The key under which the weather API key is stored.
pub const API_KEY_NAME: &str = "openweather_api_key";

/// Fetch the secret stored under the given key (if any).
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretFetchResponse)]
pub struct FetchSecret(pub String);

/// Store the second value under the first key, replacing any existing value.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretStoreResponse)]
pub struct StoreSecret(pub String, pub String);

/// Delete the secret stored under the given key.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretDeleteResponse)]
pub struct DeleteSecret(pub String);

/// The output of a [`FetchSecret`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretFetchResponse {
    /// No secret is stored under this key.
    Missing(String),
    /// The stored secret value.
    Fetched(String),
}

/// The output of a [`StoreSecret`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretStoreResponse {
    /// The secret was stored successfully under `key`.
    Stored(String),
    /// Storage failed; the string carries the error message.
    StoreError(String),
}

/// The output of a [`DeleteSecret`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretDeleteResponse {
    /// The secret was removed.
    Deleted(String),
    /// Deletion failed; the string carries the error message.
    DeleteError(String),
}
