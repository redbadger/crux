//! Resolve secret requests using localStorage.
//!
//! Web browsers don't have a secure secrets store, so localStorage is the
//! closest available approximation. Values are stored in plaintext and
//! accessible to any script on the same origin.
//!
//! Each operation has its own output type, so there is no wide response enum
//! and no way to answer one question with another's answer.

use leptos::prelude::*;

use crux_core::Request;
use shared::ViewModel;
use shared::effects::secret::{
    Delete, Fetch, SecretDeleteResponse, SecretFetchResponse, SecretStoreResponse, Store,
};

pub(super) fn fetch(
    core: &super::Core,
    mut request: Request<Fetch>,
    render: WriteSignal<ViewModel>,
) {
    let key = &request.operation.0;
    log::debug!("secret fetch: {key}");
    let response = local_storage()
        .and_then(|s| s.get_item(key).ok().flatten())
        .map_or_else(
            || {
                log::debug!("secret not found: {key}");
                SecretFetchResponse::Missing(key.clone())
            },
            |value| {
                log::debug!("secret found: {key}");
                SecretFetchResponse::Fetched(value)
            },
        );
    super::resolve_effect(core, &mut request, response, render);
}

pub(super) fn store(
    core: &super::Core,
    mut request: Request<Store>,
    render: WriteSignal<ViewModel>,
) {
    let Store(key, value) = &request.operation;
    log::debug!("secret store: {key}");
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(key, value);
    }
    let response = SecretStoreResponse::Stored(key.clone());
    super::resolve_effect(core, &mut request, response, render);
}

pub(super) fn delete(
    core: &super::Core,
    mut request: Request<Delete>,
    render: WriteSignal<ViewModel>,
) {
    let key = &request.operation.0;
    log::debug!("secret delete: {key}");
    if let Some(storage) = local_storage() {
        let _ = storage.remove_item(key);
    }
    let response = SecretDeleteResponse::Deleted(key.clone());
    super::resolve_effect(core, &mut request, response, render);
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}
