import App
import Foundation
import os
import Shared
import WeatherKit

private let logger = Logger(subsystem: "com.crux.examples.weather", category: "live-bridge")

/// Wraps `CoreFfi` to communicate with the Rust core. Handles bincode
/// serialization/deserialization so that `Core` works with Swift types only.
/// This lives in the app target (not WeatherKit) so that SwiftUI previews
/// don't need to load the Rust framework.
struct LiveBridge: CoreBridge {
    private let ffi: CoreFfi

    init() {
        ffi = CoreFfi()
    }

    func processEvent(_ event: Event) -> [Request] {
        let eventBytes = try! event.bincodeSerialize() // swiftlint:disable:this force_try
        logger.debug("sending \(eventBytes.count) event bytes")

        let effects = [UInt8](ffi.update(Data(eventBytes)))
        logger.debug("received \(effects.count) effect bytes")

        return deserializeRequests(effects)
    }

    func resolve(requestId: UInt32, responseBytes: [UInt8]) -> [Request] {
        logger.debug("resolve: id=\(requestId) sending \(responseBytes.count) bytes")

        let effects = [UInt8](ffi.resolve(requestId, Data(responseBytes)))
        return deserializeRequests(effects)
    }

    func currentView() -> ViewModel {
        // swiftlint:disable:next force_try
        try! .bincodeDeserialize(input: [UInt8](ffi.view()))
    }

    private func deserializeRequests(_ bytes: [UInt8]) -> [Request] {
        if bytes.isEmpty { return [] }
        if bytes.count < 8 {
            logger.error("response too short (\(bytes.count) bytes)")
            return []
        }
        return try! .bincodeDeserialize(input: bytes) // swiftlint:disable:this force_try
    }
}
