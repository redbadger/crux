import App
import Foundation
import Shared
import os.log

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    private let logger = Logger(subsystem: "com.crux.examples.notes", category: "Core")
    private var core: CoreFfi
    private var timerTasks: [UInt64: Task<Void, Never>] = [:]

    init() {
        logger.info("Initializing Core")
        self.core = CoreFfi()
        // swiftlint:disable:next force_try
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
    }

    func update(_ event: Event) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.update(Data(try! event.bincodeSerialize())))
        // swiftlint:disable:next force_try
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            handleRender()
        case let .keyValue(keyValue):
            handleKeyValue(request, keyValue)
        case let .time(timeReq):
            handleTime(request, timeReq)
        case let .pubSub(.publish(bytes)):
            logger.debug("PubSub publish: \(bytes.count) bytes")
        case .pubSub(.subscribe):
            logger.debug("PubSub subscribe (no backend)")
        }
    }

    private func resolveEffects(_ requestId: UInt32, _ data: Data) {
        let effects = [UInt8](core.resolve(requestId, data))
        // swiftlint:disable:next force_try
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    private func handleRender() {
        DispatchQueue.main.async {
            // swiftlint:disable:next force_try
            self.view = try! .bincodeDeserialize(input: [UInt8](self.core.view()))
        }
    }

    private func handleKeyValue(_ request: Request, _ keyValue: KeyValueOperation) {
        logger.debug("Processing KeyValue effect")
        Task {
            let result = processKeyValueOperation(keyValue)
            // swiftlint:disable:next force_try
            let data = Data(try! result.bincodeSerialize())
            resolveEffects(request.id, data)
        }
    }

    private func processKeyValueOperation(_ keyValue: KeyValueOperation) -> KeyValueResult {
        let defaults = UserDefaults.standard

        switch keyValue {
        case .get(let key):
            let value = defaults.data(forKey: key) ?? Data()
            return .ok(response: .get(value: .bytes([UInt8](value))))

        case .set(let key, let value):
            let previousData = defaults.data(forKey: key) ?? Data()
            defaults.set(Data(value), forKey: key)
            return .ok(response: .set(previous: .bytes([UInt8](previousData))))

        case .delete(let key):
            let previousData = defaults.data(forKey: key) ?? Data()
            defaults.removeObject(forKey: key)
            return .ok(response: .delete(previous: .bytes([UInt8](previousData))))

        case .exists(let key):
            let exists = defaults.object(forKey: key) != nil
            return .ok(response: .exists(isPresent: exists))

        case .listKeys(let prefix, let cursor):
            let allKeys = defaults.dictionaryRepresentation().keys
            let keys = allKeys.filter { $0.hasPrefix(prefix) }
            _ = cursor
            return .ok(response: .listKeys(keys: Array(keys), nextCursor: 0))
        }
    }

    private func handleTime(_ request: Request, _ timeReq: TimeRequest) {
        switch timeReq {
        case .notifyAfter(let id, let duration):
            logger.debug("Timer \(id) set for \(duration.nanos)ns")

            let task = Task {
                try? await Task.sleep(nanoseconds: duration.nanos)
                guard !Task.isCancelled else { return }
                let response = TimeResponse.durationElapsed(id: id)
                // swiftlint:disable:next force_try
                let data = Data(try! response.bincodeSerialize())
                self.resolveEffects(request.id, data)
            }
            timerTasks[id] = task

        case .clear(let id):
            logger.debug("Timer \(id) cleared")
            timerTasks[id]?.cancel()
            timerTasks.removeValue(forKey: id)
        }
    }
}
