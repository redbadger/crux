import SwiftUI

@main
struct WeatherApp: App {
    var body: some Scene {
        WindowGroup {
            HomeView(core: Core())
        }
    }
}

