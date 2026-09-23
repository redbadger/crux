import App
import Foundation

/// A `KeyValueHandler` over a `UserDefaults` suite. Each value is stored as
/// `Data`.
///
/// `UserDefaults` suits small values such as the favourites. It reads and
/// writes a whole domain at once, so keep larger data in a file.
///
/// `@unchecked`, because `UserDefaults` is thread-safe but is not marked as
/// `Sendable`.
nonisolated final class UserDefaultsKeyValueHandler: KeyValueHandler, @unchecked Sendable {
    private let defaults: UserDefaults
    private let suiteName: String

    init(suiteName: String) {
        self.suiteName = suiteName
        defaults = UserDefaults(suiteName: suiteName) ?? .standard
    }

    func get(_ operation: GetValue) async -> ValueResult {
        .ok(value(operation.key))
    }

    func set(_ operation: SetValue) async -> ValueResult {
        let previous = value(operation.key)
        defaults.set(Data(operation.value), forKey: operation.key)
        return .ok(previous)
    }

    func delete(_ operation: DeleteValue) async -> ValueResult {
        let previous = value(operation.key)
        defaults.removeObject(forKey: operation.key)
        return .ok(previous)
    }

    func exists(_ operation: KeyExists) async -> ExistsResult {
        .ok(defaults.data(forKey: operation.key) != nil)
    }

    /// Reads only the suite's own domain, because a read through `defaults`
    /// also finds the keys of the global domain.
    func listKeys(_ operation: ListKeys) async -> KeysResult {
        let keys = (defaults.persistentDomain(forName: suiteName) ?? [:]).keys
            .filter { $0.hasPrefix(operation.prefix) }
            .sorted()
        guard operation.cursor <= UInt64(keys.count) else {
            return .err(.cursorNotFound)
        }
        return .ok(KeyPage(keys: Array(keys[Int(operation.cursor)...]), nextCursor: 0))
    }

    private func value(_ key: String) -> Value {
        guard let data = defaults.data(forKey: key) else { return Value.none }
        return .bytes([UInt8](data))
    }
}
