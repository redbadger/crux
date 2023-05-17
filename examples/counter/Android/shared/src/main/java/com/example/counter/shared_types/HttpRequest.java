package com.example.counter.shared_types;


public final class HttpRequest {
    public final String method;
    public final String url;
    public final java.util.List<HttpHeader> headers;
    public final java.util.List<@com.novi.serde.Unsigned Byte> body;

    public HttpRequest(String method, String url, java.util.List<HttpHeader> headers, java.util.List<@com.novi.serde.Unsigned Byte> body) {
        java.util.Objects.requireNonNull(method, "method must not be null");
        java.util.Objects.requireNonNull(url, "url must not be null");
        java.util.Objects.requireNonNull(headers, "headers must not be null");
        java.util.Objects.requireNonNull(body, "body must not be null");
        this.method = method;
        this.url = url;
        this.headers = headers;
        this.body = body;
    }

    public void serialize(com.novi.serde.Serializer serializer) throws com.novi.serde.SerializationError {
        serializer.increase_container_depth();
        serializer.serialize_str(method);
        serializer.serialize_str(url);
        TraitHelpers.serialize_vector_HttpHeader(headers, serializer);
        TraitHelpers.serialize_vector_u8(body, serializer);
        serializer.decrease_container_depth();
    }

    public byte[] bincodeSerialize() throws com.novi.serde.SerializationError {
        com.novi.serde.Serializer serializer = new com.novi.bincode.BincodeSerializer();
        serialize(serializer);
        return serializer.get_bytes();
    }

    public static HttpRequest deserialize(com.novi.serde.Deserializer deserializer) throws com.novi.serde.DeserializationError {
        deserializer.increase_container_depth();
        Builder builder = new Builder();
        builder.method = deserializer.deserialize_str();
        builder.url = deserializer.deserialize_str();
        builder.headers = TraitHelpers.deserialize_vector_HttpHeader(deserializer);
        builder.body = TraitHelpers.deserialize_vector_u8(deserializer);
        deserializer.decrease_container_depth();
        return builder.build();
    }

    public static HttpRequest bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
        if (input == null) {
             throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
        }
        com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
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
        if (!java.util.Objects.equals(this.headers, other.headers)) { return false; }
        if (!java.util.Objects.equals(this.body, other.body)) { return false; }
        return true;
    }

    public int hashCode() {
        int value = 7;
        value = 31 * value + (this.method != null ? this.method.hashCode() : 0);
        value = 31 * value + (this.url != null ? this.url.hashCode() : 0);
        value = 31 * value + (this.headers != null ? this.headers.hashCode() : 0);
        value = 31 * value + (this.body != null ? this.body.hashCode() : 0);
        return value;
    }

    public static final class Builder {
        public String method;
        public String url;
        public java.util.List<HttpHeader> headers;
        public java.util.List<@com.novi.serde.Unsigned Byte> body;

        public HttpRequest build() {
            return new HttpRequest(
                method,
                url,
                headers,
                body
            );
        }
    }
}
