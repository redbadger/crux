import Serde


indirect public enum Effect: Hashable {
    case render(SharedTypes.RenderOperation)
    case http(SharedTypes.HttpRequest)
    case serverSentEvents(SharedTypes.SseRequest)

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .render(let x):
            try serializer.serialize_variant_index(value: 0)
            try x.serialize(serializer: serializer)
        case .http(let x):
            try serializer.serialize_variant_index(value: 1)
            try x.serialize(serializer: serializer)
        case .serverSentEvents(let x):
            try serializer.serialize_variant_index(value: 2)
            try x.serialize(serializer: serializer)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Effect {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try SharedTypes.RenderOperation.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .render(x)
        case 1:
            let x = try SharedTypes.HttpRequest.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .http(x)
        case 2:
            let x = try SharedTypes.SseRequest.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .serverSentEvents(x)
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for Effect: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> Effect {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum Event: Hashable {
    case get
    case increment
    case decrement
    case startWatch

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .get:
            try serializer.serialize_variant_index(value: 0)
        case .increment:
            try serializer.serialize_variant_index(value: 1)
        case .decrement:
            try serializer.serialize_variant_index(value: 2)
        case .startWatch:
            try serializer.serialize_variant_index(value: 3)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Event {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            try deserializer.decrease_container_depth()
            return .get
        case 1:
            try deserializer.decrease_container_depth()
            return .increment
        case 2:
            try deserializer.decrease_container_depth()
            return .decrement
        case 3:
            try deserializer.decrease_container_depth()
            return .startWatch
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for Event: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> Event {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum HttpError: Hashable {
    case url(String)
    case io(String)
    case timeout

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .url(let x):
            try serializer.serialize_variant_index(value: 0)
            try serializer.serialize_str(value: x)
        case .io(let x):
            try serializer.serialize_variant_index(value: 1)
            try serializer.serialize_str(value: x)
        case .timeout:
            try serializer.serialize_variant_index(value: 2)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> HttpError {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .url(x)
        case 1:
            let x = try deserializer.deserialize_str()
            try deserializer.decrease_container_depth()
            return .io(x)
        case 2:
            try deserializer.decrease_container_depth()
            return .timeout
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for HttpError: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> HttpError {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct HttpHeader: Hashable {
    @Indirect public var name: String
    @Indirect public var value: String

    public init(name: String, value: String) {
        self.name = name
        self.value = value
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_str(value: self.name)
        try serializer.serialize_str(value: self.value)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> HttpHeader {
        try deserializer.increase_container_depth()
        let name = try deserializer.deserialize_str()
        let value = try deserializer.deserialize_str()
        try deserializer.decrease_container_depth()
        return HttpHeader.init(name: name, value: value)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> HttpHeader {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct HttpRequest: Hashable {
    @Indirect public var method: String
    @Indirect public var url: String
    @Indirect public var headers: [SharedTypes.HttpHeader]
    @Indirect public var body: [UInt8]

    public init(method: String, url: String, headers: [SharedTypes.HttpHeader], body: [UInt8]) {
        self.method = method
        self.url = url
        self.headers = headers
        self.body = body
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_str(value: self.method)
        try serializer.serialize_str(value: self.url)
        try serialize_vector_HttpHeader(value: self.headers, serializer: serializer)
        try serializer.serialize_bytes(value: self.body)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> HttpRequest {
        try deserializer.increase_container_depth()
        let method = try deserializer.deserialize_str()
        let url = try deserializer.deserialize_str()
        let headers = try deserialize_vector_HttpHeader(deserializer: deserializer)
        let body = try deserializer.deserialize_bytes()
        try deserializer.decrease_container_depth()
        return HttpRequest.init(method: method, url: url, headers: headers, body: body)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> HttpRequest {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct HttpResponse: Hashable {
    @Indirect public var status: UInt16
    @Indirect public var headers: [SharedTypes.HttpHeader]
    @Indirect public var body: [UInt8]

    public init(status: UInt16, headers: [SharedTypes.HttpHeader], body: [UInt8]) {
        self.status = status
        self.headers = headers
        self.body = body
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_u16(value: self.status)
        try serialize_vector_HttpHeader(value: self.headers, serializer: serializer)
        try serializer.serialize_bytes(value: self.body)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> HttpResponse {
        try deserializer.increase_container_depth()
        let status = try deserializer.deserialize_u16()
        let headers = try deserialize_vector_HttpHeader(deserializer: deserializer)
        let body = try deserializer.deserialize_bytes()
        try deserializer.decrease_container_depth()
        return HttpResponse.init(status: status, headers: headers, body: body)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> HttpResponse {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum HttpResult: Hashable {
    case ok(SharedTypes.HttpResponse)
    case err(SharedTypes.HttpError)

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .ok(let x):
            try serializer.serialize_variant_index(value: 0)
            try x.serialize(serializer: serializer)
        case .err(let x):
            try serializer.serialize_variant_index(value: 1)
            try x.serialize(serializer: serializer)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> HttpResult {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try SharedTypes.HttpResponse.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .ok(x)
        case 1:
            let x = try SharedTypes.HttpError.deserialize(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .err(x)
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for HttpResult: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> HttpResult {
        let deserializer = BincodeDeserializer.init(input: input);
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

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> RenderOperation {
        try deserializer.increase_container_depth()
        try deserializer.decrease_container_depth()
        return RenderOperation.init()
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> RenderOperation {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct Request: Hashable {
    @Indirect public var id: UInt32
    @Indirect public var effect: SharedTypes.Effect

    public init(id: UInt32, effect: SharedTypes.Effect) {
        self.id = id
        self.effect = effect
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_u32(value: self.id)
        try self.effect.serialize(serializer: serializer)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> Request {
        try deserializer.increase_container_depth()
        let id = try deserializer.deserialize_u32()
        let effect = try SharedTypes.Effect.deserialize(deserializer: deserializer)
        try deserializer.decrease_container_depth()
        return Request.init(id: id, effect: effect)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> Request {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct SseRequest: Hashable {
    @Indirect public var url: String

    public init(url: String) {
        self.url = url
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_str(value: self.url)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> SseRequest {
        try deserializer.increase_container_depth()
        let url = try deserializer.deserialize_str()
        try deserializer.decrease_container_depth()
        return SseRequest.init(url: url)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> SseRequest {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

indirect public enum SseResponse: Hashable {
    case chunk([UInt8])
    case done

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        switch self {
        case .chunk(let x):
            try serializer.serialize_variant_index(value: 0)
            try serialize_vector_u8(value: x, serializer: serializer)
        case .done:
            try serializer.serialize_variant_index(value: 1)
        }
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> SseResponse {
        let index = try deserializer.deserialize_variant_index()
        try deserializer.increase_container_depth()
        switch index {
        case 0:
            let x = try deserialize_vector_u8(deserializer: deserializer)
            try deserializer.decrease_container_depth()
            return .chunk(x)
        case 1:
            try deserializer.decrease_container_depth()
            return .done
        default: throw DeserializationError.invalidInput(issue: "Unknown variant index for SseResponse: \(index)")
        }
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> SseResponse {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

public struct ViewModel: Hashable {
    @Indirect public var text: String
    @Indirect public var confirmed: Bool

    public init(text: String, confirmed: Bool) {
        self.text = text
        self.confirmed = confirmed
    }

    public func serialize<S: Serializer>(serializer: S) throws {
        try serializer.increase_container_depth()
        try serializer.serialize_str(value: self.text)
        try serializer.serialize_bool(value: self.confirmed)
        try serializer.decrease_container_depth()
    }

    public func bincodeSerialize() throws -> [UInt8] {
        let serializer = BincodeSerializer.init();
        try self.serialize(serializer: serializer)
        return serializer.get_bytes()
    }

    public static func deserialize<D: Deserializer>(deserializer: D) throws -> ViewModel {
        try deserializer.increase_container_depth()
        let text = try deserializer.deserialize_str()
        let confirmed = try deserializer.deserialize_bool()
        try deserializer.decrease_container_depth()
        return ViewModel.init(text: text, confirmed: confirmed)
    }

    public static func bincodeDeserialize(input: [UInt8]) throws -> ViewModel {
        let deserializer = BincodeDeserializer.init(input: input);
        let obj = try deserialize(deserializer: deserializer)
        if deserializer.get_buffer_offset() < input.count {
            throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
        }
        return obj
    }
}

func serialize_vector_HttpHeader<S: Serializer>(value: [SharedTypes.HttpHeader], serializer: S) throws {
    try serializer.serialize_len(value: value.count)
    for item in value {
        try item.serialize(serializer: serializer)
    }
}

func deserialize_vector_HttpHeader<D: Deserializer>(deserializer: D) throws -> [SharedTypes.HttpHeader] {
    let length = try deserializer.deserialize_len()
    var obj : [SharedTypes.HttpHeader] = []
    for _ in 0..<length {
        obj.append(try SharedTypes.HttpHeader.deserialize(deserializer: deserializer))
    }
    return obj
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

