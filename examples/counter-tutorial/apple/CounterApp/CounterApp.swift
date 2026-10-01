import SwiftUI

@main
struct CounterApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView(core: CoreWrapper())
        }
    }
}
