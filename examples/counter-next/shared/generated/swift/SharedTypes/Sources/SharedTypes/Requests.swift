import Serde


public extension [Request] {
  static func bincodeDeserialize(input: [UInt8]) throws -> [Request] {
    let deserializer = BincodeDeserializer(input: input)
    try deserializer.increase_container_depth()
    let length = try deserializer.deserialize_len()

    var requests: [Request] = []
    for _ in 0 ..< length {
      while deserializer.get_buffer_offset() < input.count {
        let req = try Request.deserialize(deserializer: deserializer)
        requests.append(req)
      }
    }
    deserializer.decrease_container_depth()

    return requests
  }
}
