use shared::kv::{
    KeyValueOperation, KeyValueResponse, KeyValueResult,
    value::Value,
};

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn handle(operation: &KeyValueOperation) -> KeyValueResult {
    match operation {
        KeyValueOperation::Get { key } => {
            let storage = get_local_storage();
            let value = storage
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            KeyValueResult::Ok {
                response: KeyValueResponse::Get { value },
            }
        }
        KeyValueOperation::Set { key, value } => {
            let storage = get_local_storage();
            let previous = storage
                .as_ref()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            let value_str = std::str::from_utf8(value).unwrap_or("");
            if let Some(s) = &storage {
                let _ = s.set_item(key, value_str);
            }
            KeyValueResult::Ok {
                response: KeyValueResponse::Set { previous },
            }
        }
        KeyValueOperation::Delete { key } => {
            let storage = get_local_storage();
            let previous = storage
                .as_ref()
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(v.into_bytes()))
                .unwrap_or(Value::Bytes(vec![]));
            if let Some(s) = &storage {
                let _ = s.remove_item(key);
            }
            KeyValueResult::Ok {
                response: KeyValueResponse::Delete { previous },
            }
        }
        KeyValueOperation::Exists { key } => {
            let storage = get_local_storage();
            let is_present = storage
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .is_some();
            KeyValueResult::Ok {
                response: KeyValueResponse::Exists { is_present },
            }
        }
        KeyValueOperation::ListKeys { prefix, cursor } => {
            let storage = get_local_storage();
            let mut keys = Vec::new();
            if let Some(s) = storage {
                let len = s.length().unwrap_or(0);
                for i in 0..len {
                    if let Ok(Some(key)) = s.key(i) {
                        if key.starts_with(prefix) {
                            keys.push(key);
                        }
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
