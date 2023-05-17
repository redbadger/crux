package com.example.counter.shared_types;


public abstract class Effect {

    abstract public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError;

    public static Effect deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        int index = deserializer.deserialize_variant_index();
        switch (index) {
            case 0: return Http.load(deserializer);
            case 1: return Render.load(deserializer);
            case 2: return ServerSentEvents.load(deserializer);
            default: throw new com.novi.serde.DeserializationError("Unknown variant index for Effect: " + index);
        }
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Effect bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        Effect value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public static final class Http extends Effect {
        public final HttpRequest value;

        public Http(HttpRequest value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(0);
            value.serialize(serializer);
            serializer.decrease_container_depth();
        }

        static Http load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = HttpRequest.deserialize(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Http other = (Http) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public HttpRequest value;

            public Http build() {
                return new Http(
                    value
                );
            }
        }
    }

    public static final class Render extends Effect {
        public final RenderOperation value;

        public Render(RenderOperation value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(1);
            value.serialize(serializer);
            serializer.decrease_container_depth();
        }

        static Render load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = RenderOperation.deserialize(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Render other = (Render) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public RenderOperation value;

            public Render build() {
                return new Render(
                    value
                );
            }
        }
    }

    public static final class ServerSentEvents extends Effect {
        public final SseRequest value;

        public ServerSentEvents(SseRequest value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(2);
            value.serialize(serializer);
            serializer.decrease_container_depth();
        }

        static ServerSentEvents load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = SseRequest.deserialize(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            ServerSentEvents other = (ServerSentEvents) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public SseRequest value;

            public ServerSentEvents build() {
                return new ServerSentEvents(
                    value
                );
            }
        }
    }
}

