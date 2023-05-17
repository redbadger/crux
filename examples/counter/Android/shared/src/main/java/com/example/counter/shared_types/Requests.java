package com.example.counter.shared_types;

public final class Requests {

  public static java.util.List<Request> bincodeDeserialize(byte[] input) throws com.novi.serde.DeserializationError {
    if (input == null) {
      throw new com.novi.serde.DeserializationError("Cannot deserialize null array");
    }
    com.novi.serde.Deserializer deserializer = new com.novi.bincode.BincodeDeserializer(input);
    deserializer.increase_container_depth();

    long length = deserializer.deserialize_len();

    java.util.List<Request> value = new java.util.ArrayList<>();

    for (int i = 0; i < length; i++) {
      value.add(Request.deserialize(deserializer));
    }

    deserializer.decrease_container_depth();

    if (deserializer.get_buffer_offset() < input.length) {
      throw new com.novi.serde.DeserializationError("Some input bytes were not read");
    }
    return value;
  }
}
