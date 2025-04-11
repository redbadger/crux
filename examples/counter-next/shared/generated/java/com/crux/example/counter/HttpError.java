package com.crux.example.counter;


public abstract class HttpError {

    abstract public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError;

    public static HttpError deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        int index = deserializer.deserialize_variant_index();
        switch (index) {
            case 0: return Url.load(deserializer);
            case 1: return Io.load(deserializer);
            case 2: return Timeout.load(deserializer);
            default: throw new com.novi.serde.DeserializationError("Unknown variant index for HttpError: " + index);
        }
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static HttpError bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        HttpError value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public static final class Url extends HttpError {
        public final String value;

        public Url(String value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(0);
            serializer.serialize_str(value);
            serializer.decrease_container_depth();
        }

        static Url load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = deserializer.deserialize_str();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Url other = (Url) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public String value;

            public Url build() {
                return new Url(
                    value
                );
            }
        }
    }

    public static final class Io extends HttpError {
        public final String value;

        public Io(String value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(1);
            serializer.serialize_str(value);
            serializer.decrease_container_depth();
        }

        static Io load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = deserializer.deserialize_str();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Io other = (Io) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public String value;

            public Io build() {
                return new Io(
                    value
                );
            }
        }
    }

    public static final class Timeout extends HttpError {
        public Timeout() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(2);
            serializer.decrease_container_depth();
        }

        static Timeout load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Timeout other = (Timeout) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Timeout build() {
                return new Timeout(
                );
            }
        }
    }
}

