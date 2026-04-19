import App
import Foundation

private let logger = Log.core

@Observable
@MainActor
public class Core {
    public var view: ViewModel

    private let bridge: CoreBridge
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
        let requests = bridge.processEvent(event)
        for request in requests {
            processEffect(request)
        }
    }

    // ANCHOR: dispatch
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            view = bridge.currentView()
        case let .time(timeRequest):
            resolveTime(request: timeRequest, requestId: request.id)
        case let .secret(secretRequest):
            resolveSecret(request: secretRequest, requestId: request.id)
        case let .http(httpRequest):
            resolveHttp(request: httpRequest, requestId: request.id)
        case let .keyValue(kvRequest):
            resolveKeyValue(request: kvRequest, requestId: request.id)
        case let .location(locationRequest):
            resolveLocation(request: locationRequest, requestId: request.id)
        }
    }
    // ANCHOR_END: dispatch

    // ANCHOR: resolve_helper
    func resolve(requestId: UInt32, serialize: () throws -> [UInt8]) {
        let responseBytes = try! serialize() // swiftlint:disable:this force_try
        let requests = bridge.resolve(requestId: requestId, responseBytes: responseBytes)
        for request in requests {
            processEffect(request)
        }
    }
    // ANCHOR_END: resolve_helper
}
