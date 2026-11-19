import App
import SwiftUI
import WeatherKit

@main
struct WeatherApp: App {
    @State private var core: Core
    private let updater: CoreUpdater

    // ANCHOR: start
    init() {
        let core = Core(
            handler: GroupedEffectHandler(
                network: SessionNetworkHandler(),
                store: DeviceStoreHandler(),
                time: TaskSleepTimeHandler(),
                ungrouped: CoreLocationHandler()
            )
        )

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
