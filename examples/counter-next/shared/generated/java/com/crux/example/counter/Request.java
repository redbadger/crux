package com.crux.example.counter;


public final class Request {
    public final @com.novi.serde.Unsigned Integer id;
    public final Effect effect;

    public Request(@com.novi.serde.Unsigned Integer id, Effect effect) {
        java.util.Objects.requireNonNull(id, "id must not be null");
        java.util.Objects.requireNonNull(effect, "effect must not be null");
        this.id = id;
        this.effect = effect;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_u32(id);
        effect.serialize(serializer);
        serializer.decrease_container_depth();
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Request deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.id = deserializer.deserialize_u32();
        builder.effect = Effect.deserialize(deserializer);
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static Request bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
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
        if (!java.util.Objects.equals(this.id, other.id)) { return false; }
        if (!java.util.Objects.equals(this.effect, other.effect)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.id != null ? this.id.hashCode() : 0);
        value = 31 * value + (this.effect != null ? this.effect.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public @com.novi.serde.Unsigned Integer id;
        public Effect effect;

        public Request build() {
            return new Request(
                id,
                effect
            );
        }
    }
}
