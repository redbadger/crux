use shared::kv::{KeyValueOperation, KeyValueResponse, KeyValueResult, value::Value};

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn handle(operation: &KeyValueOperation) -> KeyValueResult {
    match operation {
        KeyValueOperation::Get { key } => {
            let storage = get_local_storage();
            let value = storage
                .and_then(|s| s.get_item(key).ok())
                .flatten()
                .map_or(Value::Bytes(vec![]), |v| Value::Bytes(v.into_bytes()));
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
                .map_or(Value::Bytes(vec![]), |v| Value::Bytes(v.into_bytes()));
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
                .map_or(Value::Bytes(vec![]), |v| Value::Bytes(v.into_bytes()));
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
