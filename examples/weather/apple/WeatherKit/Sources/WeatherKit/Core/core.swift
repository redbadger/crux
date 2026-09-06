import App
import Foundation

private nonisolated let logger = Log.core

@Observable
@MainActor
public class Core {
    public var view: ViewModel

    private let bridge: CoreBridge
    private var dispatcher: EffectDispatcher!
    let keyValueStore: KeyValueStore
    var activeTimers: [UInt64: Timer] = [:]

    public init(bridge: CoreBridge) {
        logger.info("Initializing Core")
        self.bridge = bridge

        do {
            self.keyValueStore = try KeyValueStore()
            logger.debug("KeyValueStore initialized successfully")
        } catch {
            logger.error("Failed to initialize KeyValueStore: \(error.localizedDescription)")
            fatalError("KeyValueStore initialization failed: \(error)")
        }

        view = bridge.currentView()

        // ANCHOR: resolve_helper
        // The dispatcher hands us the serialized output of a handler method,
        // from whichever task ran it, and we feed it back to the core.
        dispatcher = EffectDispatcher(handler: self) { [weak self] requestId, responseBytes in
            Task { @MainActor [weak self] in
                guard let self else { return }
                self.process(self.bridge.resolve(requestId: requestId, responseBytes: responseBytes))
            }
        }
        // ANCHOR_END: resolve_helper
    }

    #if DEBUG
        public static func forPreviewing(view: ViewModel) -> Core {
            Core(bridge: FakeBridge(view: view))
        }

        public static func forPreviewing() -> Core {
            forPreviewing(view: .loading)
        }
    #endif

    public func update(_ event: Event) {
        process(bridge.processEvent(event))
    }

    // ANCHOR: dispatch
    /// Every request goes to the generated `EffectDispatcher`, which calls the
    /// matching `EffectHandler` method and resolves the request for us — never
    /// for a notification, exactly once for a request.
    private func process(_ requests: [Request]) {
        for request in requests {
            dispatcher.dispatch(request)
        }
    }
    // ANCHOR_END: dispatch

    /// Re-read the view model. `render` is a notification, so there is nothing
    /// to resolve.
    func refreshView() {
        view = bridge.currentView()
    }
}
