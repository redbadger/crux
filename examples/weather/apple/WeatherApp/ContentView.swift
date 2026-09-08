import SwiftUI
import WeatherKit

struct ContentView: View {
    @Environment(ViewStore.self) var store

    var body: some View {
        switch store.view {
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
