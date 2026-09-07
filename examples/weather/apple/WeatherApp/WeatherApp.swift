import App
import SwiftUI
import WeatherKit

@main
struct WeatherApp: App {
    @State private var store: ViewStore
    private let core: Core
    private let updater: CoreUpdater

    // ANCHOR: start
    init() {
        let store = ViewStore()
        let core = Core(bridge: LiveBridge(), handler: WeatherHandler()) { store.view = $0 }

        _store = State(wrappedValue: store)
        self.core = core
        updater = CoreUpdater { core.update($0) }

        core.update(.start)
    }
    // ANCHOR_END: start

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(store)
                .environment(updater)
        }
    }
}
