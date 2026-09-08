//! A custom capability for storing and retrieving secrets (e.g. API keys).
//!
//! Three operations — [`Fetch`], [`Store`] and [`Delete`] — each with its own
//! output type naming only the outcomes that operation can actually have:
//! [`SecretFetchResponse`], [`SecretStoreResponse`] and
//! [`SecretDeleteResponse`]. There is no wide response enum shared between
//! them, so no call site has to rule out variants that cannot happen. The
//! developer-facing command builders live in the [`command`] submodule.

pub mod command;

use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// The key under which the weather API key is stored.
pub const API_KEY_NAME: &str = "openweather_api_key";

/// Fetch the secret stored under the given key (if any).
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretFetchResponse)]
pub struct Fetch(pub String);

/// Store the second value under the first key, replacing any existing value.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretStoreResponse)]
pub struct Store(pub String, pub String);

/// Delete the secret stored under the given key.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[operation(request, output = SecretDeleteResponse)]
pub struct Delete(pub String);

/// The output of a [`Fetch`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretFetchResponse {
    /// No secret is stored under this key.
    Missing(String),
    /// The stored secret value.
    Fetched(String),
}

/// The output of a [`Store`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretStoreResponse {
    /// The secret was stored successfully under `key`.
    Stored(String),
    /// Storage failed; the string carries the error message.
    StoreError(String),
}

/// The output of a [`Delete`].
#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretDeleteResponse {
    /// The secret was removed.
    Deleted(String),
    /// Deletion failed; the string carries the error message.
    DeleteError(String),
}
