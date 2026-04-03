pub mod command;

use crux_core::capability::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

pub const API_KEY_NAME: &str = "openweather_api_key";

#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretRequest {
    Fetch(String),
    Store(String, String),
    Delete(String),
}

impl Operation for SecretRequest {
    type Output = SecretResponse;
}

#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretResponse {
    Missing(String),
    Fetched(String, String),
    Stored(String),
    StoreError(String),
    Deleted(String),
    DeleteError(String),
}

#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretFetchResponse {
    Missing(String),
    Fetched(String),
}

#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretStoreResponse {
    Stored(String),
    StoreError(String),
}

#[derive(Facet, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[repr(C)]
pub enum SecretDeleteResponse {
    Deleted(String),
    DeleteError(String),
}
