package com.redbadger.catfacts.shared_types;


public final class ViewModel {
    public final String fact;
    public final java.util.Optional<CatImage> image;
    public final String platform;

    public ViewModel(String fact, java.util.Optional<CatImage> image, String platform) {
        java.util.Objects.requireNonNull(fact, "fact must not be null");
        java.util.Objects.requireNonNull(image, "image must not be null");
        java.util.Objects.requireNonNull(platform, "platform must not be null");
        this.fact = fact;
        this.image = image;
        this.platform = platform;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(fact);
        TraitHelpers.serialize_option_CatImage(image, serializer);
        serializer.serialize_str(platform);
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
        builder.fact = deserializer.deserialize_str();
        builder.image = TraitHelpers.deserialize_option_CatImage(deserializer);
        builder.platform = deserializer.deserialize_str();
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
        if (!java.util.Objects.equals(this.fact, other.fact)) { return false; }
        if (!java.util.Objects.equals(this.image, other.image)) { return false; }
        if (!java.util.Objects.equals(this.platform, other.platform)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.fact != null ? this.fact.hashCode() : 0);
        value = 31 * value + (this.image != null ? this.image.hashCode() : 0);
        value = 31 * value + (this.platform != null ? this.platform.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String fact;
        public java.util.Optional<CatImage> image;
        public String platform;

        public ViewModel build() {
            return new ViewModel(
                fact,
                image,
                platform
            );
        }
    }
}
