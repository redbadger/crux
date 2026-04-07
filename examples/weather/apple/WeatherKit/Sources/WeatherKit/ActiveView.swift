import App
import SwiftUI

public struct ActiveView: View {
    let model: ActiveViewModel

    public init(model: ActiveViewModel) {
        self.model = model
    }

    public var body: some View {
        NavigationStack {
            ZStack {
                Color.systemGroupedBackground
                    .ignoresSafeArea()

                switch model {
                case let .home(home):
                    HomeView(model: home)
                        .transition(
                            .opacity.combined(with: .offset(x: 0, y: 10))
                        )
                case let .favorites(favorites):
                    FavoritesView(model: favorites)
                        .transition(
                            .opacity.combined(with: .offset(x: 0, y: 10))
                        )
                }
            }
            .animation(.easeOut(duration: 0.2), value: model)
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
