use facet::Facet;
use serde::{Deserialize, Serialize};

/// The value stored under a key.
///
/// `Value::None` is used to represent the absence of a value.
///
/// Note: we can't use `Option` here because generics are not currently
/// supported across the FFI boundary, when using the builtin typegen.
#[derive(Facet, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum Value {
    None,
    Bytes(#[serde(with = "serde_bytes")] Vec<u8>),
}

impl From<Vec<u8>> for Value {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }
}

impl From<Value> for Option<Vec<u8>> {
    fn from(value: Value) -> Self {
        match value {
            Value::None => None,
            Value::Bytes(bytes) => Some(bytes),
        }
    }
}

impl From<Option<Vec<u8>>> for Value {
    fn from(val: Option<Vec<u8>>) -> Self {
        val.map_or(Self::None, Value::Bytes)
    }
}
