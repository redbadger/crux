use leptos::prelude::*;

use shared::ViewModel;
use shared::kv::{KeyValueOperation, KeyValueResponse, KeyValueResult, value::Value};

pub(super) fn resolve(
    core: &super::Core,
    mut request: crux_core::Request<KeyValueOperation>,
    render: WriteSignal<ViewModel>,
) {
    let response = handle(&request.operation);

    match core.resolve(&mut request, response) {
        Ok(new_effects) => super::process_effects(core, new_effects, render),
        Err(e) => log::warn!("failed to resolve key_value: {e:?}"),
    }
}

fn local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn handle(operation: &KeyValueOperation) -> KeyValueResult {
    match operation {
        KeyValueOperation::Get { key } => {
            log::debug!("kv get: {key}");
            let value = local_storage()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            KeyValueResult::Ok {
                response: KeyValueResponse::Get { value },
            }
        }
        KeyValueOperation::Set { key, value } => {
            log::debug!("kv set: {key}");
            let previous = local_storage()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            let value_str = std::str::from_utf8(value).unwrap_or("");
            if let Some(s) = local_storage() {
                let _ = s.set_item(key, value_str);
            }
            KeyValueResult::Ok {
                response: KeyValueResponse::Set { previous },
            }
        }
        KeyValueOperation::Delete { key } => {
            log::debug!("kv delete: {key}");
            let previous = local_storage()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            if let Some(s) = local_storage() {
                let _ = s.remove_item(key);
            }
            KeyValueResult::Ok {
                response: KeyValueResponse::Delete { previous },
            }
        }
        KeyValueOperation::Exists { key } => {
            log::debug!("kv exists: {key}");
            let is_present = local_storage()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .is_some();
            KeyValueResult::Ok {
                response: KeyValueResponse::Exists { is_present },
            }
        }
        KeyValueOperation::ListKeys { prefix, cursor } => {
            log::debug!("kv list_keys: prefix={prefix}");
            let mut keys = Vec::new();
            if let Some(s) = local_storage() {
                let len = s.length().unwrap_or(0);
                for i in 0..len {
                    if let Ok(Some(key)) = s.key(i)
                        && key.starts_with(prefix)
                    {
                        keys.push(key);
                    }
                }
            }
            let _ = cursor;
            KeyValueResult::Ok {
                response: KeyValueResponse::ListKeys {
                    keys,
                    next_cursor: 0,
                },
            }
        }
    }
}
