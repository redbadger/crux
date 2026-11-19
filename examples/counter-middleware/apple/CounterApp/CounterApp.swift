import App
import SwiftUI

@main
struct CounterApp: App {
    @State private var core = makeCore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environment(core)
        }
    }
}
