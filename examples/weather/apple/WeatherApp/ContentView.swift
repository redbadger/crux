import SwiftUI
import WeatherKit

struct ContentView: View {
    @Environment(Core.self) var core

    var body: some View {
        switch core.view {
        case .loading:
            ProgressView("Loading...")

        case let .onboard(onboard):
            OnboardView(model: onboard)

        case let .active(active):
            ActiveView(model: active)

        case let .failed(message):
            FailedView(message: message)
        }
    }
}
