package com.example.counter.shared_types;


public final class ViewModel {
    public final String text;
    public final Boolean confirmed;

    public ViewModel(String text, Boolean confirmed) {
        java.util.Objects.requireNonNull(text, "text must not be null");
        java.util.Objects.requireNonNull(confirmed, "confirmed must not be null");
        this.text = text;
        this.confirmed = confirmed;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(text);
        serializer.serialize_bool(confirmed);
        serializer.decrease_container_depth();
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static ViewModel deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.text = deserializer.deserialize_str();
        builder.confirmed = deserializer.deserialize_bool();
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static ViewModel bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        ViewModel value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        ViewModel other = (ViewModel) obj;
        if (!java.util.Objects.equals(this.text, other.text)) { return false; }
        if (!java.util.Objects.equals(this.confirmed, other.confirmed)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.text != null ? this.text.hashCode() : 0);
        value = 31 * value + (this.confirmed != null ? this.confirmed.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String text;
        public Boolean confirmed;

        public ViewModel build() {
            return new ViewModel(
                text,
                confirmed
            );
        }
    }
}
