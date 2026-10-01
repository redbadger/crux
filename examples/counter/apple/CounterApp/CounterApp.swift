import App
import SwiftUI

@main
struct CounterApp: App {
    @State private var core = Core(handler: CounterHandler())

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(core)
        }
    }
}
