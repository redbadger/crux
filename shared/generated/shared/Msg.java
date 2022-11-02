package shared;


public abstract class Msg {

    abstract public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError;

    public static Msg deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        int index = deserializer.deserialize_variant_index();
        switch (index) {
            case 0: return None.load(deserializer);
            case 1: return GetPlatform.load(deserializer);
            case 2: return SetPlatform.load(deserializer);
            case 3: return Clear.load(deserializer);
            case 4: return Get.load(deserializer);
            case 5: return Fetch.load(deserializer);
            case 6: return Restore.load(deserializer);
            case 7: return SetState.load(deserializer);
            case 8: return SetFact.load(deserializer);
            case 9: return SetImage.load(deserializer);
            case 10: return CurrentTime.load(deserializer);
            default: throw new com.novi.serde.DeserializationError("Unknown variant index for Msg: " + index);
        }
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static Msg bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
        Msg value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public static final class None extends Msg {
        public None() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(0);
            serializer.decrease_container_depth();
        }

        static None load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            None other = (None) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public None build() {
                return new None(
                );
            }
        }
    }

    public static final class GetPlatform extends Msg {
        public GetPlatform() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(1);
            serializer.decrease_container_depth();
        }

        static GetPlatform load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            GetPlatform other = (GetPlatform) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public GetPlatform build() {
                return new GetPlatform(
                );
            }
        }
    }

    public static final class SetPlatform extends Msg {
        public final String platform;

        public SetPlatform(String platform) {
            java.util.Objects.requireNonNull(platform, "platform must not be null");
            this.platform = platform;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(2);
            serializer.serialize_str(platform);
            serializer.decrease_container_depth();
        }

        static SetPlatform load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.platform = deserializer.deserialize_str();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            SetPlatform other = (SetPlatform) obj;
            if (!java.util.Objects.equals(this.platform, other.platform)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.platform != null ? this.platform.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public String platform;

            public SetPlatform build() {
                return new SetPlatform(
                    platform
                );
            }
        }
    }

    public static final class Clear extends Msg {
        public Clear() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(3);
            serializer.decrease_container_depth();
        }

        static Clear load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Clear other = (Clear) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Clear build() {
                return new Clear(
                );
            }
        }
    }

    public static final class Get extends Msg {
        public Get() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(4);
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

    public static final class Fetch extends Msg {
        public Fetch() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(5);
            serializer.decrease_container_depth();
        }

        static Fetch load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Fetch other = (Fetch) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Fetch build() {
                return new Fetch(
                );
            }
        }
    }

    public static final class Restore extends Msg {
        public Restore() {
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(6);
            serializer.decrease_container_depth();
        }

        static Restore load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            Restore other = (Restore) obj;
            return true;
        }

        public int hashCode() {
            int value = 7;
            return value;
        }

        public static final class Builder {
            public Restore build() {
                return new Restore(
                );
            }
        }
    }

    public static final class SetState extends Msg {
        public final java.util.Optional<java.util.List<@com.novi.serde.Unsigned Byte>> bytes;

        public SetState(java.util.Optional<java.util.List<@com.novi.serde.Unsigned Byte>> bytes) {
            java.util.Objects.requireNonNull(bytes, "bytes must not be null");
            this.bytes = bytes;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(7);
            TraitHelpers.serialize_option_vector_u8(bytes, serializer);
            serializer.decrease_container_depth();
        }

        static SetState load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.bytes = TraitHelpers.deserialize_option_vector_u8(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            SetState other = (SetState) obj;
            if (!java.util.Objects.equals(this.bytes, other.bytes)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.bytes != null ? this.bytes.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public java.util.Optional<java.util.List<@com.novi.serde.Unsigned Byte>> bytes;

            public SetState build() {
                return new SetState(
                    bytes
                );
            }
        }
    }

    public static final class SetFact extends Msg {
        public final java.util.List<@com.novi.serde.Unsigned Byte> bytes;

        public SetFact(java.util.List<@com.novi.serde.Unsigned Byte> bytes) {
            java.util.Objects.requireNonNull(bytes, "bytes must not be null");
            this.bytes = bytes;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(8);
            TraitHelpers.serialize_vector_u8(bytes, serializer);
            serializer.decrease_container_depth();
        }

        static SetFact load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.bytes = TraitHelpers.deserialize_vector_u8(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            SetFact other = (SetFact) obj;
            if (!java.util.Objects.equals(this.bytes, other.bytes)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.bytes != null ? this.bytes.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public java.util.List<@com.novi.serde.Unsigned Byte> bytes;

            public SetFact build() {
                return new SetFact(
                    bytes
                );
            }
        }
    }

    public static final class SetImage extends Msg {
        public final java.util.List<@com.novi.serde.Unsigned Byte> bytes;

        public SetImage(java.util.List<@com.novi.serde.Unsigned Byte> bytes) {
            java.util.Objects.requireNonNull(bytes, "bytes must not be null");
            this.bytes = bytes;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(9);
            TraitHelpers.serialize_vector_u8(bytes, serializer);
            serializer.decrease_container_depth();
        }

        static SetImage load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.bytes = TraitHelpers.deserialize_vector_u8(deserializer);
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            SetImage other = (SetImage) obj;
            if (!java.util.Objects.equals(this.bytes, other.bytes)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.bytes != null ? this.bytes.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public java.util.List<@com.novi.serde.Unsigned Byte> bytes;

            public SetImage build() {
                return new SetImage(
                    bytes
                );
            }
        }
    }

    public static final class CurrentTime extends Msg {
        public final String iso_time;

        public CurrentTime(String iso_time) {
            java.util.Objects.requireNonNull(iso_time, "iso_time must not be null");
            this.iso_time = iso_time;
        }

        public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
            serializer.increase_container_depth();
            serializer.serialize_variant_index(10);
            serializer.serialize_str(iso_time);
            serializer.decrease_container_depth();
        }

        static CurrentTime load(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
            deserializer.increase_container_depth();
            Builder builder = new Builder();
            builder.iso_time = deserializer.deserialize_str();
            deserializer.decrease_container_depth();
            return builder.build();
        }

        public boolean equals(Object obj) {
            if (this == obj) return true;
            if (obj == null) return false;
            if (getClass() != obj.getClass()) return false;
            CurrentTime other = (CurrentTime) obj;
            if (!java.util.Objects.equals(this.iso_time, other.iso_time)) { return false; }
            return true;
        }

        public int hashCode() {
            int value = 7;
            value = 31 * value + (this.iso_time != null ? this.iso_time.hashCode() : 0);
            return value;
        }

        public static final class Builder {
            public String iso_time;

            public CurrentTime build() {
                return new CurrentTime(
                    iso_time
                );
            }
        }
    }
}

