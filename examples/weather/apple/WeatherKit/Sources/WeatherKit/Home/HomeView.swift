import App
import SwiftUI

// ANCHOR: home_view
struct HomeView: View {
    @Environment(CoreUpdater.self) var update
    let model: HomeViewModel
    @State private var selectedPage = 0

    var body: some View {
        VStack {
            TabView(selection: $selectedPage) {
                // Local weather card
                localWeatherCard
                    .tag(0)
                    .tabItem { Label("Current", systemImage: "location") }

                // Favorite weather cards
                ForEach(Array(model.favorites.enumerated()), id: \.element.name) { idx, favorite in
                    favoriteWeatherCard(favorite)
                        .tag(idx + 1)
                        .tabItem { Label(favorite.name, systemImage: "star") }
                }
            }
            #if os(iOS)
            .tabViewStyle(PageTabViewStyle(indexDisplayMode: .automatic))
            #endif
        }
        .padding(.vertical)
        .toolbar {
            ToolbarItem(placement: .automatic) {
                Button {
                    update(.active(.home(.goToFavorites)))
                } label: {
                    Image(systemName: "star")
                }
            }
            ToolbarItem(placement: .automatic) {
                Button {
                    update(.active(.resetApiKey))
                } label: {
                    Image(systemName: "key")
                }
            }
        }
    }

    @ViewBuilder
    private var localWeatherCard: some View {
        switch model.localWeather {
        case .checkingPermission:
            StatusCard(message: "Checking location permission...")
        case .locationDisabled:
            StatusCard(message: "Location is disabled", icon: "location.slash")
        case .fetchingLocation:
            StatusCard(message: "Getting your location...")
        case .fetchingWeather:
            LoadingCard()
        case let .fetched(weatherData):
            WeatherCard(weatherData: weatherData)
                .transition(.opacity)
        case .failed:
            StatusCard(message: "Failed to load weather", icon: "exclamationmark.triangle")
        }
    }

    @ViewBuilder
    private func favoriteWeatherCard(_ favorite: FavoriteWeatherViewModel) -> some View {
        switch favorite.weather {
        case .fetching:
            LoadingCard()
        case let .fetched(weatherData):
            WeatherCard(weatherData: weatherData)
                .transition(.opacity)
        case .failed:
            StatusCard(message: "Failed to load weather for \(favorite.name)", icon: "exclamationmark.triangle")
        }
    }
}
// ANCHOR_END: home_view

#Preview {
    HomeView(model: HomeViewModel(
        localWeather: .fetched(previewWeatherResponse),
        favorites: []
    ))
    .previewEnvironment()
}
