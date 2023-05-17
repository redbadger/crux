package com.example.counter.shared_types;


public abstract class Event {

    abstract public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError;

    public static Event deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        int index = deserializer.deserialize_variant_index();
        switch (index) {
            case 0: return Get.load(deserializer);
            case 1: return Increment.load(deserializer);
            case 2: return Decrement.load(deserializer);
            case 3: return StartWatch.load(deserializer);
            case 4: return SendUuid.load(deserializer);
            case 5: return WatchUpdate.load(deserializer);
            default: throw new com.novi.serde.DeserializationError("Unknown variant index for Event: " + index);
        }
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Event bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        Event value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public static final class Get extends Event {
        public Get() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(0);
            serializer.decrease_container_depth();
        }

        static Get load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Get other = (Get) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Get build() {
                return new Get(
                );
            }
        }
    }

    public static final class Increment extends Event {
        public Increment() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(1);
            serializer.decrease_container_depth();
        }

        static Increment load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Increment other = (Increment) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Increment build() {
                return new Increment(
                );
            }
        }
    }

    public static final class Decrement extends Event {
        public Decrement() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(2);
            serializer.decrease_container_depth();
        }

        static Decrement load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Decrement other = (Decrement) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Decrement build() {
                return new Decrement(
                );
            }
        }
    }

    public static final class StartWatch extends Event {
        public StartWatch() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(3);
            serializer.decrease_container_depth();
        }

        static StartWatch load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            StartWatch other = (StartWatch) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public StartWatch build() {
                return new StartWatch(
                );
            }
        }
    }

    public static final class SendUuid extends Event {
        public final com.novi.serde.Bytes value;

        public SendUuid(com.novi.serde.Bytes value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(4);
            serializer.serialize_bytes(value);
            serializer.decrease_container_depth();
        }

        static SendUuid load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = deserializer.deserialize_bytes();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            SendUuid other = (SendUuid) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public com.novi.serde.Bytes value;

            public SendUuid build() {
                return new SendUuid(
                    value
                );
            }
        }
    }

    public static final class WatchUpdate extends Event {
        public final Counter value;

        public WatchUpdate(Counter value) {
            java.util.Objects.requireNonNull(value, "value must not be null");
            this.value = value;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(5);
            value.serialize(serializer);
            serializer.decrease_container_depth();
        }

        static WatchUpdate load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.value = Counter.deserialize(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            WatchUpdate other = (WatchUpdate) obj;
            if (!java.util.Objects.equals(this.value, other.value)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.value != null ? this.value.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public Counter value;

            public WatchUpdate build() {
                return new WatchUpdate(
                    value
                );
            }
        }
    }
}

