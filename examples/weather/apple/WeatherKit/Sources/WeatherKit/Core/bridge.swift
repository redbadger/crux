import App
import Foundation

/// Abstraction over the Rust FFI boundary. Production uses `LiveBridge` (defined in the app);
/// previews use `FakeBridge` to avoid loading the Rust framework.
public protocol CoreBridge {
    func processEvent(_ event: Event) -> [Request]
    func resolve(requestId: UInt32, responseBytes: [UInt8]) -> [Request]
    func currentView() -> ViewModel
}

/// No-op bridge for SwiftUI previews. Returns a static view model
/// and ignores all events.
#if DEBUG
    public struct FakeBridge: CoreBridge {
        let view: ViewModel

        public init(view: ViewModel) {
            self.view = view
        }

        public func processEvent(_: Event) -> [Request] { [] }
        public func resolve(requestId _: UInt32, responseBytes _: [UInt8]) -> [Request] { [] }
        public func currentView() -> ViewModel { view }
    }
#endif
