use leptos::prelude::*;

use crux_core::Request;
use shared::ViewModel;
use shared::kv::{operation, value::Value};

/// `Get` is answered with a `ValueResult` — nothing else will do.
pub(super) fn get(
    core: &super::Core,
    mut request: Request<operation::Get>,
    render: WriteSignal<ViewModel>,
) {
    let key = &request.operation.key;
    log::debug!("kv get: {key}");
    let response = operation::ValueResult::Ok(read(key));
    super::resolve_effect(core, &mut request, response, render);
}

/// `Set` is answered with the value it replaced.
pub(super) fn set(
    core: &super::Core,
    mut request: Request<operation::Set>,
    render: WriteSignal<ViewModel>,
) {
    let operation::Set { key, value } = &request.operation;
    log::debug!("kv set: {key}");
    let previous = read(key);
    let value_str = std::str::from_utf8(value).unwrap_or("");
    if let Some(storage) = local_storage() {
        let _ = storage.set_item(key, value_str);
    }
    let response = operation::ValueResult::Ok(previous);
    super::resolve_effect(core, &mut request, response, render);
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn read(key: &str) -> Value {
    local_storage()
        .and_then(|s| s.get_item(key).ok())
        .flatten()
        .map_or(Value::Bytes(vec![]), |v| Value::Bytes(v.into_bytes()))
}
