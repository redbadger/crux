import App
import SwiftUI

enum LocationSelection: Hashable {
    case current
    case favorite(Location)
}

// ANCHOR: home_view
struct HomeView: View {
    @Environment(CoreUpdater.self) var update
    let model: HomeViewModel
    @State private var selection: LocationSelection? = .current
    @State private var showResetConfirmation = false

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                CurrentLocationRow(localWeather: model.localWeather)
                    .tag(LocationSelection.current)

                ForEach(model.favorites, id: \.location) { favorite in
                    LocationRow(
                        name: favorite.name,
                        weather: favorite.weather,
                        isCurrentLocation: false
                    )
                    .tag(LocationSelection.favorite(favorite.location))
                }
            }
            .navigationTitle("Weather")
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    Button {
                        update(.active(.home(.goToFavorites)))
                    } label: {
                        Image(systemName: "list.bullet")
                    }
                }
                ToolbarItem(placement: .automatic) {
                    Button {
                        showResetConfirmation = true
                    } label: {
                        Image(systemName: "gearshape")
                    }
                }
            }
            .confirmationDialog("Settings", isPresented: $showResetConfirmation) {
                Button("Reset API Key", role: .destructive) {
                    update(.active(.resetApiKey))
                }
            }
        } detail: {
            detailView
        }
        .navigationSplitViewStyle(.prominentDetail)
    }

    @ViewBuilder
    private var detailView: some View {
        switch selection {
        case .current:
            currentLocationDetail
        case let .favorite(location):
            if let favorite = model.favorites.first(where: { $0.location == location }) {
                favoriteDetail(favorite)
            } else {
                ContentUnavailableView("Select a location", systemImage: "mappin.and.ellipse")
            }
        case nil:
            ContentUnavailableView("Select a location", systemImage: "mappin.and.ellipse")
        }
    }

    @ViewBuilder
    private var currentLocationDetail: some View {
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
            ScrollView {
                WeatherCard(weatherData: weatherData)
            }
        case .failed:
            StatusCard(message: "Failed to load weather", icon: "exclamationmark.triangle")
        }
    }

    @ViewBuilder
    private func favoriteDetail(_ favorite: FavoriteWeatherViewModel) -> some View {
        switch favorite.weather {
        case .fetching:
            LoadingCard()
        case let .fetched(weatherData):
            ScrollView {
                WeatherCard(weatherData: weatherData)
            }
        case .failed:
            StatusCard(message: "Failed to load weather for \(favorite.name)", icon: "exclamationmark.triangle")
        }
    }
}
// ANCHOR_END: home_view

#Preview {
    HomeView(model: HomeViewModel(
        localWeather: .fetched(previewWeatherResponse),
        favorites: [
            FavoriteWeatherViewModel(
                name: "Paris",
                location: Location(lat: 48.8566, lon: 2.3522),
                weather: .fetched(previewWeatherResponse)
            )
        ]
    ))
    .previewEnvironment()
}
