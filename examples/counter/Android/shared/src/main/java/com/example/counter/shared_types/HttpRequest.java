package com.example.counter.shared_types;


public final class HttpRequest {
    public final String method;
    public final String url;

    public HttpRequest(String method, String url) {
        java.util.Objects.requireNonNull(method, "method must not be null");
        java.util.Objects.requireNonNull(url, "url must not be null");
        this.method = method;
        this.url = url;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(method);
        serializer.serialize_str(url);
        serializer.decrease_container_depth();
    }

    public byte[] bcsSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bcs.BcsSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static HttpRequest deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.method = deserializer.deserialize_str();
        builder.url = deserializer.deserialize_str();
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static HttpRequest bcsDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bcs.BcsDeserializer(input);
        HttpRequest value = deserialize(deserializer);
        if (deserializer.get_buffer_offset() < input.length) {
             throw new com.novi.serde.DeserializationError("Some input bytes were not read");
        }
        return value;
    }

    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null) return false;
        if (getClass() != obj.getClass()) return false;
        HttpRequest other = (HttpRequest) obj;
        if (!java.util.Objects.equals(this.method, other.method)) { return false; }
        if (!java.util.Objects.equals(this.url, other.url)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.method != null ? this.method.hashCode() : 0);
        value = 31 * value + (this.url != null ? this.url.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String method;
        public String url;

        public HttpRequest build() {
            return new HttpRequest(
                method,
                url
            );
        }
    }
}
