import SwiftUI
import WeatherKit

@main
struct WeatherApp: App {
    @State private var core: Core
    private let updater: CoreUpdater

    // ANCHOR: start
    init() {
        let bridge = LiveBridge()
        let core = Core(bridge: bridge)
        _core = State(wrappedValue: core)
        updater = CoreUpdater { core.update($0) }
        core.update(.start)
    }
    // ANCHOR_END: start

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(core)
                .environment(updater)
        }
    }
}
