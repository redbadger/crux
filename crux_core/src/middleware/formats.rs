use bincode::{
    config::{AllowTrailing, FixintEncoding, WithOtherIntEncoding, WithOtherTrailing},
    de::read::SliceReader,
    DefaultOptions, Options as _,
};

use super::FfiFormat;

pub struct BincodeFfiFormat;

impl BincodeFfiFormat {
    fn bincode_options(
    ) -> WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>
    {
        DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes()
    }
}

impl FfiFormat for BincodeFfiFormat {
    type Serializer<'b> = bincode::Serializer<
        &'b mut Vec<u8>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    >;
    type Deserializer<'b> = bincode::Deserializer<
        SliceReader<'b>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    >;

    fn serializer(
        buffer: &mut Vec<u8>,
    ) -> bincode::Serializer<
        &'_ mut Vec<u8>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    > {
        bincode::Serializer::new(buffer, Self::bincode_options())
    }

    fn deserializer(
        bytes: &[u8],
    ) -> bincode::Deserializer<
        SliceReader<'_>,
        WithOtherTrailing<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, AllowTrailing>,
    > {
        bincode::Deserializer::from_slice(bytes, Self::bincode_options())
    }
}
