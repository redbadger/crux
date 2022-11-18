package com.redbadger.crux_core.shared_types;

final class TraitHelpers {
    static void serialize_option_CatImage(java.util.Optional<CatImage> value, com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        if (value.isPresent()) {
            serializer.serialize_option_tag(true);
            value.get().serialize(serializer);
        } else {
            serializer.serialize_option_tag(false);
        }
    }

    static java.util.Optional<CatImage> deserialize_option_CatImage(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        boolean tag = deserializer.deserialize_option_tag();
        if (!tag) {
            return java.util.Optional.empty();
        } else {
            return java.util.Optional.of(CatImage.deserialize(deserializer));
        }
    }

    static void serialize_option_vector_u8(java.util.Optional<java.util.List<@com.novi.serde.Unsigned Byte>> value, com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        if (value.isPresent()) {
            serializer.serialize_option_tag(true);
            TraitHelpers.serialize_vector_u8(value.get(), serializer);
        } else {
            serializer.serialize_option_tag(false);
        }
    }

    static java.util.Optional<java.util.List<@com.novi.serde.Unsigned Byte>> deserialize_option_vector_u8(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        boolean tag = deserializer.deserialize_option_tag();
        if (!tag) {
            return java.util.Optional.empty();
        } else {
            return java.util.Optional.of(TraitHelpers.deserialize_vector_u8(deserializer));
        }
    }

    static void serialize_vector_u8(java.util.List<@com.novi.serde.Unsigned Byte> value, com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.serialize_len(value.size());
        for (@com.novi.serde.Unsigned Byte item : value) {
            serializer.serialize_u8(item);
        }
    }

    static java.util.List<@com.novi.serde.Unsigned Byte> deserialize_vector_u8(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        long length = deserializer.deserialize_len();
        java.util.List<@com.novi.serde.Unsigned Byte> obj = new java.util.ArrayList<@com.novi.serde.Unsigned Byte>((int) length);
        for (long i = 0; i < length; i++) {
            obj.add(deserializer.deserialize_u8());
        }
        return obj;
    }

}

