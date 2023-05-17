package com.example.counter.shared_types;


public final class HttpHeader {
    public final String name;
    public final String value;

    public HttpHeader(String name, String value) {
        java.util.Objects.requireNonNull(name, "name must not be null");
        java.util.Objects.requireNonNull(value, "value must not be null");
        this.name = name;
        this.value = value;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(name);
        serializer.serialize_str(value);
        serializer.decrease_container_depth();
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static HttpHeader deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.name = deserializer.deserialize_str();
        builder.value = deserializer.deserialize_str();
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static HttpHeader bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        HttpHeader value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        HttpHeader other = (HttpHeader) obj;
        if (!java.util.Objects.equals(this.name, other.name)) { return false; }
        if (!java.util.Objects.equals(this.value, other.value)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.name != null ? this.name.hashCode() : 0);
        value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String name;
        public String value;

        public HttpHeader build() {
            return new HttpHeader(
                name,
                value
            );
        }
    }
}
