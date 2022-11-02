import Serde


indirect public enum Msg: Hashable {
    case none
    case getPlatform
    case setPlatform(platform: String)
    case clear
    case get
    case fetch
    case restore
    case setState(bytes: [UInt8]?)
    case setFact(bytes: [UInt8])
    case setImage(bytes: [UInt8])
    case currentTime(iso_time: String)

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .none:
            try serializer.serialize_variant_index(value: 0)
        case .getPlatform:
            try serializer.serialize_variant_index(value: 1)
        case .setPlatform(let platform):
            try serializer.serialize_variant_index(value: 2)
            try serializer.serialize_str(value: platform)
        case .clear:
            try serializer.serialize_variant_index(value: 3)
        case .get:
            try serializer.serialize_variant_index(value: 4)
        case .fetch:
            try serializer.serialize_variant_index(value: 5)
        case .restore:
            try serializer.serialize_variant_index(value: 6)
        case .setState(let bytes):
            try serializer.serialize_variant_index(value: 7)
            try serialize_option_vector_u8(value: bytes, serializer: serializer)
        case .setFact(let bytes):
            try serializer.serialize_variant_index(value: 8)
            try serialize_vector_u8(value: bytes, serializer: serializer)
        case .setImage(let bytes):
            try serializer.serialize_variant_index(value: 9)
            try serialize_vector_u8(value: bytes, serializer: serializer)
        case .currentTime(let iso_time):
            try serializer.serialize_variant_index(value: 10)
            try serializer.serialize_str(value: iso_time)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Msg {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            try deserializer.decrease_container_depth()
            return .none
        case 1:
            try deserializer.decrease_container_depth()
            return .getPlatform
        case 2:
            let platform = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .setPlatform(platform: platform)
        case 3:
            try deserializer.decrease_container_depth()
            return .clear
        case 4:
            try deserializer.decrease_container_depth()
            return .get
        case 5:
            try deserializer.decrease_container_depth()
            return .fetch
        case 6:
            try deserializer.decrease_container_depth()
            return .restore
        case 7:
            let bytes = try deserialize_option_vector_u8(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .setState(bytes: bytes)
        case 8:
            let bytes = try deserialize_vector_u8(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .setFact(bytes: bytes)
        case 9:
            let bytes = try deserialize_vector_u8(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .setImage(bytes: bytes)
        case 10:
            let iso_time = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .currentTime(iso_time: iso_time)
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for Msg: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> Msg {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

func serialize_option_vector_u8<S: Serializer>(value: [UInt8]?, serializer: S) throws {
    if let value = value {
        try serializer.serialize_option_tag(value: true)
        try serialize_vector_u8(value: value, serializer: serializer)
    } else {
        try serializer.serialize_option_tag(value: false)
    }
}

func deserialize_option_vector_u8<D: Deserializer>(deserializer: D) throws -> [UInt8]? {
    let tag = try deserializer.deserialize_option_tag()
    if tag {
        return try deserialize_vector_u8(deserializer: deserializer)
    } else {
        return nil
    }
}

func serialize_vector_u8<S: Serializer>(value: [UInt8], serializer: S) throws {
    try serializer.serialize_len(value: value.count)
    for item in value {
        try serializer.serialize_u8(value: item)
    }
}

func deserialize_vector_u8<D: Deserializer>(deserializer: D) throws -> [UInt8] {
    let length = try deserializer.deserialize_len()
    var obj : [UInt8] = []
    for _ in 0..<length {
        obj.append(try deserializer.deserialize_u8())
    }
    return obj
}

