import App
import Foundation
import Shared

/// The generated `CoreBridge`, implemented over BoltFFI's `CoreFfi`: bytes in,
/// bytes out, nothing else. This is the only file in the app that knows the
/// Rust core exists, and it lives in the app target (not WeatherKit) so that
/// SwiftUI previews don't need to load the Rust framework.
///
/// `nonisolated` because the target builds with `MainActor` as its default
/// isolation, and `CoreBridge`'s requirements are not actor-isolated.
/// `@unchecked Sendable` because `CoreFfi` is a class Swift cannot prove
/// `Sendable`; its state is a handle into a mutex-guarded `Bridge` on the Rust
/// side, so calling it from any task is safe.
nonisolated struct LiveBridge: CoreBridge, @unchecked Sendable {
    private let ffi = CoreFfi()

    func update(_ event: [UInt8]) -> [UInt8] {
        [UInt8](ffi.update(data: Data(event)))
    }

    func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8] {
        [UInt8](ffi.resolve(id: id, data: Data(output)))
    }

    func view() -> [UInt8] {
        [UInt8](ffi.view())
    }
}
