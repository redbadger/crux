import SwiftUI

@main
struct SimpleCounterApp: App {
    var body: some Scene {
        WindowGroup {
            ContentView(model: Core())
        }
    }
}
