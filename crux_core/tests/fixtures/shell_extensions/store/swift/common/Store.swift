import Foundation

public protocol StoreHandler: Sendable {
    func get(_ operation: Get) async -> StoreValue
}

public struct InMemoryStoreHandler: StoreHandler {
    public init() {}

    public func get(_ operation: Get) async -> StoreValue {
        StoreValue(value: Array(operation.key.utf8))
    }
}
