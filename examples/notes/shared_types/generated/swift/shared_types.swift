import Serde


indirect public enum Effect: Hashable {
    case render(RenderOperation)

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .render(let x):
            try serializer.serialize_variant_index(value: 0)
            try x.serialize(serializer: serializer)
        }
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Effect {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try RenderOperation.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .render(x)
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for Effect: \(index)")
        }
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> Effect {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum Event: Hashable {
    case insert(String)
    case replace(UInt64, UInt64, String)
    case moveCursor(UInt64)
    case select(UInt64, UInt64)
    case backspace
    case delete

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .insert(let x):
            try serializer.serialize_variant_index(value: 0)
            try serializer.serialize_str(value: x)
        case .replace(let x0, let x1, let x2):
            try serializer.serialize_variant_index(value: 1)
            try serializer.serialize_u64(value: x0)
            try serializer.serialize_u64(value: x1)
            try serializer.serialize_str(value: x2)
        case .moveCursor(let x):
            try serializer.serialize_variant_index(value: 2)
            try serializer.serialize_u64(value: x)
        case .select(let x0, let x1):
            try serializer.serialize_variant_index(value: 3)
            try serializer.serialize_u64(value: x0)
            try serializer.serialize_u64(value: x1)
        case .backspace:
            try serializer.serialize_variant_index(value: 4)
        case .delete:
            try serializer.serialize_variant_index(value: 5)
        }
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Event {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .insert(x)
        case 1:
            let x0 = try deserializer.deserialize_u64()
            let x1 = try deserializer.deserialize_u64()
            let x2 = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .replace(x0, x1, x2)
        case 2:
            let x = try deserializer.deserialize_u64()
            try deserializer.decrease_container_depth()
            return .moveCursor(x)
        case 3:
            let x0 = try deserializer.deserialize_u64()
            let x1 = try deserializer.deserialize_u64()
            try deserializer.decrease_container_depth()
            return .select(x0, x1)
        case 4:
            try deserializer.decrease_container_depth()
            return .backspace
        case 5:
            try deserializer.decrease_container_depth()
            return .delete
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for Event: \(index)")
        }
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> Event {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct Range: Hashable {
    @Indirect public var start: UInt64
    @Indirect public var end: UInt64

    public init(start: UInt64, end: UInt64) {
        self.start = start
        self.end = end
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_u64(value: self.start)
        try serializer.serialize_u64(value: self.end)
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Range {
        try deserializer.increase_container_depth()
        let start = try deserializer.deserialize_u64()
        let end = try deserializer.deserialize_u64()
        try deserializer.decrease_container_depth()
        return Range.init(start: start, end: end)
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> Range {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct RenderOperation: Hashable {

    public init() {
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> RenderOperation {
        try deserializer.increase_container_depth()
        try deserializer.decrease_container_depth()
        return RenderOperation.init()
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> RenderOperation {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct Request: Hashable {
    @Indirect public var uuid: [UInt8]
    @Indirect public var effect: Effect

    public init(uuid: [UInt8], effect: Effect) {
        self.uuid = uuid
        self.effect = effect
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serialize_vector_u8(value: self.uuid, serializer: serializer)
        try self.effect.serialize(serializer: serializer)
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Request {
        try deserializer.increase_container_depth()
        let uuid = try deserialize_vector_u8(deserializer: deserializer)
        let effect = try Effect.deserialize(deserializer: deserializer)
        try deserializer.decrease_container_depth()
        return Request.init(uuid: uuid, effect: effect)
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> Request {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum TextCursor: Hashable {
    case position(UInt64)
    case selection(Range)

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .position(let x):
            try serializer.serialize_variant_index(value: 0)
            try serializer.serialize_u64(value: x)
        case .selection(let x):
            try serializer.serialize_variant_index(value: 1)
            try x.serialize(serializer: serializer)
        }
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> TextCursor {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try deserializer.deserialize_u64()
            try deserializer.decrease_container_depth()
            return .position(x)
        case 1:
            let x = try Range.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .selection(x)
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for TextCursor: \(index)")
        }
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> TextCursor {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct ViewModel: Hashable {
    @Indirect public var text: String
    @Indirect public var cursor: TextCursor

    public init(text: String, cursor: TextCursor) {
        self.text = text
        self.cursor = cursor
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_str(value: self.text)
        try self.cursor.serialize(serializer: serializer)
        try serializer.decrease_container_depth()
    }

    public func bcsSerialize() throws -> [UInt8] {
        let serializer = BcsSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> ViewModel {
        try deserializer.increase_container_depth()
        let text = try deserializer.deserialize_str()
        let cursor = try TextCursor.deserialize(deserializer: deserializer)
        try deserializer.decrease_container_depth()
        return ViewModel.init(text: text, cursor: cursor)
    }

    public static func bcsDeserialize(input: [UInt8]) throws -> ViewModel {
        let deserializer = BcsDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
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



public extension [Request] {
  static func bcsDeserialize(input: [UInt8]) throws -> [Request] {
    let deserializer = BcsDeserializer(input: input)
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
