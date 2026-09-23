import Foundation

/// The shell side of `crux_kv`, as one method per operation.
///
/// Each method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
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
/// One store ships: `InMemoryKeyValueHandler`, which keeps values only in
/// memory. To keep values after the app stops, conform your own type to this
/// protocol over a store that persists (`UserDefaults`, a file, the keychain or
/// a database). You can also conform a type to replace a single method.
///
/// ## What each operation answers with
///
/// `get` answers with what is stored under the key, `Value.none` when nothing
/// is. `set` and `delete` answer with the value they replaced or removed, which
/// is also `Value.none` when there was none. Nothing here is an error unless
/// the store itself failed; a missing key is an answer.
public protocol KeyValueHandler: Sendable {
    /// Read the bytes stored under `operation.key`.
    func get(_ operation: GetValue) async -> ValueResult
    /// Write `operation.value`, answering with the value it replaced.
    func set(_ operation: SetValue) async -> ValueResult
    /// Remove `operation.key`, answering with the value it removed.
    func delete(_ operation: DeleteValue) async -> ValueResult
    /// Whether `operation.key` is in the store.
    func exists(_ operation: KeyExists) async -> ExistsResult
    /// The keys starting with `operation.prefix`, from `operation.cursor`.
    func listKeys(_ operation: ListKeys) async -> KeysResult
}

/// A `KeyValueHandler` that keeps values in a dictionary for the life of the
/// handler.
///
/// Use it in tests, in previews, in a shell with no storage, or when the values
/// must not outlive the process. `listKeys` answers with one page, so the next
/// cursor is always 0.
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
