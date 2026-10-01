import App

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

    /// A preview never sends an event, so the core never asks for an effect and
    /// none of these run. `render` is not here: the generated `Core` handles it.
    nonisolated struct PreviewHandler: EffectHandler {
        func http(_: HttpRequest) async -> HttpResult {
            neverRuns()
        }

        func kvGet(_: GetValue) async -> ValueResult {
            neverRuns()
        }

        func kvSet(_: SetValue) async -> ValueResult {
            neverRuns()
        }

        func timeNotifyAfter(_: NotifyAfter) async -> TimerId {
            neverRuns()
        }

        func timeClear(_: ClearTimer) async -> TimerId {
            neverRuns()
        }

        func isLocationEnabled(_: IsLocationEnabled) async -> Bool {
            neverRuns()
        }

        func getLocation(_: GetLocation) async -> Location? {
            neverRuns()
        }

        func fetchSecret(_: FetchSecret) async -> SecretFetchResponse {
            neverRuns()
        }

        func storeSecret(_: StoreSecret) async -> SecretStoreResponse {
            neverRuns()
        }

        func deleteSecret(_: DeleteSecret) async -> SecretDeleteResponse {
            neverRuns()
        }
    }

    private nonisolated func neverRuns() -> Never {
        fatalError("a preview core does not run effects")
    }

#endif
