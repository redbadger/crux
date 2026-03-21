use base64::{Engine, engine::general_purpose::STANDARD};
use shared::kv::{KeyValueOperation, KeyValueResponse, KeyValueResult, value::Value};

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn bytes_to_storage(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

fn storage_to_bytes(s: &str) -> Vec<u8> {
    STANDARD.decode(s).unwrap_or_default()
}

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn handle(operation: &KeyValueOperation) -> KeyValueResult {
    match operation {
        KeyValueOperation::Get { key } => {
            let storage = get_local_storage();
            let value = storage
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map(|v| Value::Bytes(storage_to_bytes(&v)))
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
                .map(|v| Value::Bytes(storage_to_bytes(&v)))
                .unwrap_or(Value::Bytes(vec![]));
            let encoded = bytes_to_storage(value);
            if let Some(s) = &storage {
                let _ = s.set_item(key, &encoded);
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
                .map(|v| Value::Bytes(storage_to_bytes(&v)))
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
