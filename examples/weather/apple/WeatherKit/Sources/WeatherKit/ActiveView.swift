import App
import SwiftUI

public struct ActiveView: View {
    let model: ActiveViewModel

    public init(model: ActiveViewModel) {
        self.model = model
    }

    public var body: some View {
        switch model {
        case let .home(home):
            HomeView(model: home)
        case let .favorites(favorites):
            NavigationStack {
                FavoritesView(model: favorites)
            }
        }
    }
}

#Preview {
    ActiveView(model: .home(HomeViewModel(
        localWeather: .fetched(previewWeatherResponse),
        favorites: []
    )))
    .previewEnvironment()
}
