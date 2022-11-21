package com.redbadger.crux_core.shared_types;


public final class Request {
    public final java.util.List<@com.novi.serde.Unsigned Byte> uuid;
    public final RequestBody body;

    public Request(java.util.List<@com.novi.serde.Unsigned Byte> uuid, RequestBody body) {
        java.util.Objects.requireNonNull(uuid, "uuid must not be null");
        java.util.Objects.requireNonNull(body, "body must not be null");
        this.uuid = uuid;
        this.body = body;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        TraitHelpers.serialize_vector_u8(uuid, serializer);
        body.serialize(serializer);
        serializer.decrease_container_depth();
    }

    public byte[] bcsSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bcs.BcsSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Request deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.uuid = TraitHelpers.deserialize_vector_u8(deserializer);
        builder.body = RequestBody.deserialize(deserializer);
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static Request bcsDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bcs.BcsDeserializer(input);
        Request value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        Request other = (Request) obj;
        if (!java.util.Objects.equals(this.uuid, other.uuid)) { return false; }
        if (!java.util.Objects.equals(this.body, other.body)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.uuid != null ? this.uuid.hashCode() : 0);
        value = 31 * value + (this.body != null ? this.body.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public java.util.List<@com.novi.serde.Unsigned Byte> uuid;
        public RequestBody body;

        public Request build() {
            return new Request(
                uuid,
                body
            );
        }
    }
}
