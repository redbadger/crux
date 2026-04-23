import App
import Foundation

private let logger = Log.kv

extension Core {
    func resolveKeyValue(request: KeyValueOperation, requestId: UInt32) {
        logger.debug("Processing KeyValue effect: \(String(describing: request))")
        let result = processKeyValueOperation(request)
        resolve(requestId: requestId, serialize: { try result.bincodeSerialize() })
    }

    private func processKeyValueOperation(_ keyValue: KeyValueOperation) -> KeyValueResult {
        switch keyValue {
        case let .get(key):
            logger.debug("Getting value for key: \(key)")
            let value = keyValueStore.get(key: key)
            logger.debug("Retrieved value: \(value.isEmpty ? "empty" : value)")
            let valueData = value.data(using: .utf8) ?? Data()
            return .ok(response: .get(value: .bytes([UInt8](valueData))))

        case let .set(key, value):
            logger.debug("Setting value for key: \(key)")
            let valueString = String(bytes: value, encoding: .utf8) ?? ""
            logger.debug("Value to store: \(valueString)")
            let previousValue = keyValueStore.get(key: key)
            let previousData = previousValue.data(using: .utf8) ?? Data()
            keyValueStore.set(key: key, value: valueString)
            logger.debug("Value stored successfully")
            return .ok(response: .set(previous: .bytes([UInt8](previousData))))

        case let .delete(key):
            logger.debug("Deleting key: \(key)")
            let previousValue = keyValueStore.get(key: key)
            let previousData = previousValue.data(using: .utf8) ?? Data()
            keyValueStore.delete(key: key)
            logger.debug("Key deleted successfully")
            return .ok(response: .delete(previous: .bytes([UInt8](previousData))))

        case let .exists(key):
            logger.debug("Checking existence of key: \(key)")
            let exists = keyValueStore.exists(key: key)
            logger.debug("Key exists: \(exists)")
            return .ok(response: .exists(isPresent: exists))

        case let .listKeys(prefix, cursor):
            logger.debug("Listing keys with prefix: \(prefix), cursor: \(String(describing: cursor))")
            let keys = keyValueStore.listKeys(prefix: prefix, cursor: String(cursor))
            logger.debug("Found keys: \(keys)")
            return .ok(response: .listKeys(keys: keys, nextCursor: 0))
        }
    }
}
