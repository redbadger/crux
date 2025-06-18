import SwiftUI
import SharedTypes

// View modifier to handle data state transitions
private struct WeatherDataTransitionModifier: ViewModifier {
    let weatherData: CurrentResponse?
    let favorites: [FavoriteView]
    @State private var previousWeatherData: CurrentResponse?
    @State private var previousFavorites: [FavoriteView] = []
    
    var displayWeatherData: CurrentResponse? {
        if let weatherData = weatherData, weatherData.main.temp.isFinite {
            previousWeatherData = weatherData
            previousFavorites = favorites
            return weatherData
        }
        return previousWeatherData
    }
    
    var displayFavorites: [FavoriteView] {
        if let weatherData = weatherData, weatherData.main.temp.isFinite {
            return favorites
        }
        return previousFavorites
    }
    
    func body(content: Content) -> some View {
        content
            .opacity(displayWeatherData != nil ? 1 : 0)
            .animation(.easeInOut, value: displayWeatherData != nil)
    }
}

// HomeView is the main entry point for the weather UI.
struct HomeView: View {
    @ObservedObject var core: Core
    @State private var hasLoadedInitialData = false
    @State private var selectedPage = 0
    
    var body: some View {
        Group {
            if case .home(let weatherData, let favorites) = core.view.workflow {
                VStack {
                    TabView(selection: $selectedPage) {
                        // Main weather card
                        Group {
                            if weatherData.cod == 200 && weatherData.main.temp.isFinite {
                                WeatherCard(weatherData: weatherData)
                                    .transition(.opacity)
                            } else {
                                LoadingCard()
                            }
                        }
                        .frame(width: UIScreen.main.bounds.width)
                        .tag(0)
                        
                        // Favorite weather cards
                        ForEach(Array(favorites.enumerated()), id: \.element.name) { idx, favorite in
                            Group {
                                if let current = favorite.current {
                                    WeatherCard(weatherData: current)
                                        .transition(.opacity)
                                } else {
                                    LoadingCard()
                                }
                            }
                            .frame(width: UIScreen.main.bounds.width)
                            .tag(idx + 1)
                        }
                    }
                    .tabViewStyle(PageTabViewStyle(indexDisplayMode: .automatic))
                }
                .padding(.vertical)
                .toolbar {
                    ToolbarItem(placement: .navigationBarTrailing) {
                        Button {
                            withAnimation(.easeOut(duration: 0.2)) {
                                core.update(.navigate(Workflow.favorites(FavoritesState.idle)))
                            }
                        } label: {
                            Image(systemName: "star")
                        }
                    }
                }
            } else {
                Color.clear // Placeholder for transition
            }
        }
        
        .onAppear {
            if !hasLoadedInitialData {
                core.update(.home(.show))
                hasLoadedInitialData = true
            }
        }
    }
}

// Loading placeholder card
struct LoadingCard: View {
    var body: some View {
        VStack(spacing: 20) {
            ProgressView()
            Text("Loading weather data...")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(.secondarySystemBackground))
        .cornerRadius(12)
        .padding()
    }
}

#Preview {
    HomeView(core: Core())
}
