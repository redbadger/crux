import Foundation

/// A `KeyValueHandler` that keeps values in a dictionary for the life of the
/// handler.
///
/// The app's handler holds an instance and delegates each operation to it:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let kv = InMemoryKeyValueHandler()
///
///     func kvGet(_ operation: GetValue) async -> ValueResult { await kv.get(operation) }
///     func kvSet(_ operation: SetValue) async -> ValueResult { await kv.set(operation) }
/// }
/// ```
///
/// Use it in tests, in previews, in a shell with no storage, or when the values
/// must not outlive the process. To keep values after the app stops, conform
/// your own type to `KeyValueHandler` over a store that persists.
/// `listKeys` answers with one page, so the next cursor is always 0.
///
/// A lock guards the dictionary, not an actor, because the generated types are
/// not `Sendable`.
public final class InMemoryKeyValueHandler: KeyValueHandler, @unchecked Sendable {
    private var values: [String: [UInt8]] = [:]
    private let lock = NSLock()

    public init() {}

    public func get(_ operation: GetValue) async -> ValueResult {
        .ok(Self.value(locked { values[operation.key] }))
    }

    public func set(_ operation: SetValue) async -> ValueResult {
        .ok(Self.value(locked { values.updateValue(operation.value, forKey: operation.key) }))
    }

    public func delete(_ operation: DeleteValue) async -> ValueResult {
        .ok(Self.value(locked { values.removeValue(forKey: operation.key) }))
    }

    public func exists(_ operation: KeyExists) async -> ExistsResult {
        .ok(locked { values[operation.key] != nil })
    }

    public func listKeys(_ operation: ListKeys) async -> KeysResult {
        let keys = locked { Array(values.keys) }
            .filter { $0.hasPrefix(operation.prefix) }
            .sorted()

        guard operation.cursor <= UInt64(keys.count) else {
            return .err(.cursorNotFound)
        }

        return .ok(KeyPage(keys: Array(keys[Int(operation.cursor)...]), nextCursor: 0))
    }

    private func locked<T>(_ body: () -> T) -> T {
        lock.lock()
        defer { lock.unlock() }
        return body()
    }

    /// An array is a value, so the caller gets a copy.
    private static func value(_ bytes: [UInt8]?) -> Value {
        guard let bytes else { return .none }
        return .bytes(bytes)
    }
}
