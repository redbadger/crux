use leptos::prelude::*;

use shared::ViewModel;
use shared::effects::secret::{SecretRequest, SecretResponse};

/// Resolve a secret request using localStorage.
///
/// Web browsers don't have a secure secrets store, so localStorage is the
/// closest available approximation. Values are stored in plaintext and
/// accessible to any script on the same origin.
pub(super) fn resolve(
    core: &super::Core,
    mut request: crux_core::Request<SecretRequest>,
    render: WriteSignal<ViewModel>,
) {
    let response = handle(&request.operation);
    super::resolve_effect(core, &mut request, response, render);
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn handle(operation: &SecretRequest) -> SecretResponse {
    match operation {
        SecretRequest::Fetch(key) => {
            log::debug!("secret fetch: {key}");
            match local_storage().and_then(|s| s.get_item(key).ok().flatten()) {
                Some(value) => {
                    log::debug!("secret found: {key}");
                    SecretResponse::Fetched(key.clone(), value)
                }
                None => {
                    log::debug!("secret not found: {key}");
                    SecretResponse::Missing(key.clone())
                }
            }
        }
        SecretRequest::Store(key, value) => {
            log::debug!("secret store: {key}");
            if let Some(storage) = local_storage() {
                let _ = storage.set_item(key, value);
            }
            SecretResponse::Stored(key.clone())
        }
        SecretRequest::Delete(key) => {
            log::debug!("secret delete: {key}");
            if let Some(storage) = local_storage() {
                let _ = storage.remove_item(key);
            }
            SecretResponse::Deleted(key.clone())
        }
    }
}
