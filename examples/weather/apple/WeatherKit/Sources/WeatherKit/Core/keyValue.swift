import App
import Foundation

private nonisolated let logger = Log.keyValue

/// The app's `StoreHandler`: key-value data in Core Data (here) and secrets
/// in the Keychain (`secret.swift`).
///
/// The Core Data stack is confined to the main queue, so the class is
/// `@MainActor`. `StoreHandler` is `Sendable` and its requirements are not
/// actor-isolated, so the handler methods are `nonisolated` and hop to the
/// main actor only where they touch the store.
@MainActor
public final class DeviceStoreHandler {
    let keyValueStore: KeyValueStore

    public init() {
        do {
            keyValueStore = try KeyValueStore()
            logger.debug("KeyValueStore initialized successfully")
        } catch {
            logger.error("Failed to initialize KeyValueStore: \(error.localizedDescription)")
            fatalError("KeyValueStore initialization failed: \(error)")
        }
    }
}

nonisolated extension DeviceStoreHandler: StoreHandler {
    /// `KvGet` is answered with a `ValueResult` — the value stored under the
    /// key, or the error that stopped us reading it.
    public func kvGet(_ operation: GetValue) async -> ValueResult {
        logger.debug("Getting value for key: \(operation.key)")
        let value = await read(key: operation.key)
        logger.debug("Retrieved value: \(value.isEmpty ? "empty" : value)")
        return .ok(value.asValue)
    }

    /// `KvSet` is answered with the value it replaced.
    public func kvSet(_ operation: SetValue) async -> ValueResult {
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
