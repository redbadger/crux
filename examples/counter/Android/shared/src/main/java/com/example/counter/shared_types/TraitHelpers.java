package com.example.counter.shared_types;

final class TraitHelpers {
    static void serialize_vector_HttpHeader(java.util.List<HttpHeader> value, com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.serialize_len(value.size());
        for (HttpHeader item : value) {
            item.serialize(serializer);
        }
    }

    static java.util.List<HttpHeader> deserialize_vector_HttpHeader(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        long length = deserializer.deserialize_len();
        java.util.List<HttpHeader> obj = new java.util.ArrayList<HttpHeader>((int) length);
        for (long i = 0; i < length; i++) {
            obj.add(HttpHeader.deserialize(deserializer));
        }
        return obj;
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
