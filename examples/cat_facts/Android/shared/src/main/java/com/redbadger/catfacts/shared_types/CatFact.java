package com.redbadger.catfacts.shared_types;


public final class CatFact {
    public final String fact;
    public final Integer length;

    public CatFact(String fact, Integer length) {
        java.util.Objects.requireNonNull(fact, "fact must not be null");
        java.util.Objects.requireNonNull(length, "length must not be null");
        this.fact = fact;
        this.length = length;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(fact);
        serializer.serialize_i32(length);
        serializer.decrease_container_depth();
    }

    public byte[] bcsSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bcs.BcsSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static CatFact deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.fact = deserializer.deserialize_str();
        builder.length = deserializer.deserialize_i32();
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static CatFact bcsDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bcs.BcsDeserializer(input);
        CatFact value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        CatFact other = (CatFact) obj;
        if (!java.util.Objects.equals(this.fact, other.fact)) { return false; }
        if (!java.util.Objects.equals(this.length, other.length)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.fact != null ? this.fact.hashCode() : 0);
        value = 31 * value + (this.length != null ? this.length.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String fact;
        public Integer length;

        public CatFact build() {
            return new CatFact(
                fact,
                length
            );
        }
    }
}
