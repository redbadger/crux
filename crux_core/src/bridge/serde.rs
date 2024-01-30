//! Serialization support for data across the FFI boundary.
use std::error::Error;

pub use serde::{Deserialize, Serialize};

/// A serializer for data across the FFI boundary. This allows
/// you to choose a different serialization format than bincode.
///
/// *Warning*: that support for custom serialization is *experimental* and
/// does not have a corresponding type generation support - you will need
/// to write deserialization code on the shell side yourself, or generate
/// it using other tooling.
pub trait Serializer: Clone {
    type Error: Error;

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Self::Error>
    where
        T: Serialize;

    fn deserialize<'de, T>(&self, data: &'de [u8]) -> Result<T, Self::Error>
    where
        T: Deserialize<'de>;
}

/// The default serializer for data across the FFI boundary.
#[derive(Clone)]
pub(crate) struct Bincode;

impl Serializer for Bincode {
    type Error = bincode::Error;

    fn serialize<T>(&self, data: &T) -> Result<Vec<u8>, Self::Error>
    where
        T: Serialize,
    {
        bincode::serialize(&data)
    }

    fn deserialize<'de, T>(&self, data: &'de [u8]) -> Result<T, Self::Error>
    where
        T: Deserialize<'de>,
    {
        bincode::deserialize(data)
    }
}
