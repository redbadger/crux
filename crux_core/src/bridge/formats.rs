use super::FfiFormat;

/// The default serialisation format implementation used in the FFI calls. Uses [`bincode`].
#[derive(Debug)]
pub struct BincodeFfiFormat;

impl BincodeFfiFormat {
    fn bincode_options() -> impl bincode::Options {
        use bincode::Options;
        bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }
}

impl FfiFormat for BincodeFfiFormat {
    type Error = bincode::Error;

    fn serialize<T: serde::Serialize>(buffer: &mut Vec<u8>, value: &T) -> Result<(), Self::Error> {
        value.serialize(&mut bincode::Serializer::new(
            buffer,
            Self::bincode_options(),
        ))
    }

    fn deserialize<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Self::Error> {
        T::deserialize(&mut bincode::Deserializer::from_slice(
            bytes,
            Self::bincode_options(),
        ))
    }
}

/// A JSON serialisation format implementation used in the FFI calls. Uses [`serde_json`].
#[derive(Debug)]
pub struct JsonFfiFormat;

impl FfiFormat for JsonFfiFormat {
    type Error = serde_json::Error;

    fn serialize<T: serde::Serialize>(buffer: &mut Vec<u8>, value: &T) -> Result<(), Self::Error> {
        serde_json::to_writer(buffer, value)
    }

    fn deserialize<'de, T: serde::Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Self::Error> {
        serde_json::from_slice(bytes)
    }
}
