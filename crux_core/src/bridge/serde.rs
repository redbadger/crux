use std::error::Error;

pub use serde::{Deserialize, Serialize};

/// A basic serializer for data across the FFI boundary. This allows
/// you to choose a different serialization format than bincode.
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
