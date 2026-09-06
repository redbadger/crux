import App
import Foundation

private nonisolated let logger = Log.keyValue

nonisolated extension Core {
    /// `KvGet` is answered with a `ValueResult` — the value stored under the
    /// key, or the error that stopped us reading it.
    public func kvGet(_ operation: App.Get) async -> ValueResult {
        logger.debug("Getting value for key: \(operation.key)")
        let value = await read(key: operation.key)
        logger.debug("Retrieved value: \(value.isEmpty ? "empty" : value)")
        return .ok(value.asValue)
    }

    /// `KvSet` is answered with the value it replaced.
    public func kvSet(_ operation: App.Set) async -> ValueResult {
        logger.debug("Setting value for key: \(operation.key)")
        let value = String(bytes: operation.value, encoding: .utf8) ?? ""
        let previous = await write(key: operation.key, value: value)
        logger.debug("Value stored successfully")
        return .ok(previous.asValue)
    }

    /// The Core Data stack is confined to the main queue, so the store is only
    /// touched from the main actor. Only `String`s cross back.
    @MainActor
    private func read(key: String) -> String {
        keyValueStore.get(key: key)
    }

    @MainActor
    private func write(key: String, value: String) -> String {
        let previous = keyValueStore.get(key: key)
        keyValueStore.set(key: key, value: value)
        return previous
    }
}

private nonisolated extension String {
    var asValue: Value {
        .bytes([UInt8](data(using: .utf8) ?? Data()))
    }
}
