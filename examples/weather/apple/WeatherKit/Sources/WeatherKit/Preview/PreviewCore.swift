import App
import Synchronization

#if DEBUG

    // MARK: - Preview Core

    /// A `CoreBridge` that answers `view()` with a fixed view model and ignores
    /// everything else, so a preview can build a real `Core` without the FFI.
    nonisolated struct PreviewBridge: CoreBridge {
        private let viewBytes: [UInt8]

        init(view: ViewModel = .loading) {
            // swiftlint:disable:next force_try
            viewBytes = try! view.bincodeSerialize()
        }

        func update(_: [UInt8]) -> [UInt8] {
            []
        }

        func resolve(_: UInt32, _: [UInt8]) -> [UInt8] {
            []
        }

        func view() -> [UInt8] {
            viewBytes
        }
    }

    /// The preview's `EffectHandler`. It keeps the app's real timers and puts
    /// fakes in the other groups, so a preview never uses the network, Core
    /// Data, the Keychain or CoreLocation. `render` is not here: the generated
    /// `Core` handles it.
    func previewHandler() -> GroupedEffectHandler {
        GroupedEffectHandler(
            network: OfflineNetwork(),
            store: InMemoryStore(),
            time: TaskSleepTimeHandler(),
            ungrouped: FixedLocation()
        )
    }

    /// Every request fails as if the device is offline.
    nonisolated struct OfflineNetwork: NetworkHandler {
        func http(_: HttpRequest) async -> HttpResult {
            .err(.io("previews are offline"))
        }
    }

    /// Key-value data and secrets in memory, empty at the start.
    nonisolated final class InMemoryStore: StoreHandler {
        private let values = Mutex<[String: [UInt8]]>([:])

        func kvGet(_ operation: GetValue) async -> ValueResult {
            .ok(values.withLock { $0[operation.key] }.map(Value.bytes) ?? .none)
        }

        func kvSet(_ operation: SetValue) async -> ValueResult {
            let previous = values.withLock { $0.updateValue(operation.value, forKey: operation.key) }
            return .ok(previous.map(Value.bytes) ?? .none)
        }

        func fetchSecret(_ operation: FetchSecret) async -> SecretFetchResponse {
            .missing(operation.value)
        }

        func storeSecret(_ operation: StoreSecret) async -> SecretStoreResponse {
            .stored(operation.field0)
        }

        func deleteSecret(_ operation: DeleteSecret) async -> SecretDeleteResponse {
            .deleted(operation.value)
        }
    }

    /// Location services are on, and the device is in London.
    nonisolated struct FixedLocation: UngroupedHandler {
        func isLocationEnabled(_: IsLocationEnabled) async -> Bool {
            true
        }

        func getLocation(_: GetLocation) async -> Location? {
            Location(lat: 51.5074, lon: -0.1278)
        }
    }

#endif
