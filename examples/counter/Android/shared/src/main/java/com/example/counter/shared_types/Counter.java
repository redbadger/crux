package com.example.counter.shared_types;


public final class Counter {
    public final Long value;
    public final Long updated_at;

    public Counter(Long value, Long updated_at) {
        java.util.Objects.requireNonNull(value, "value must not be null");
        java.util.Objects.requireNonNull(updated_at, "updated_at must not be null");
        this.value = value;
        this.updated_at = updated_at;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_i64(value);
        serializer.serialize_i64(updated_at);
        serializer.decrease_container_depth();
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Counter deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.value = deserializer.deserialize_i64();
        builder.updated_at = deserializer.deserialize_i64();
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static Counter bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        Counter value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        Counter other = (Counter) obj;
        if (!java.util.Objects.equals(this.value, other.value)) { return false; }
        if (!java.util.Objects.equals(this.updated_at, other.updated_at)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
        value = 31 * value + (this.updated_at != null ? this.updated_at.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public Long value;
        public Long updated_at;

        public Counter build() {
            return new Counter(
                value,
                updated_at
            );
        }
    }
}
