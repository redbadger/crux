import SwiftUI
import SharedTypes

// HomeView is the main entry point for the weather UI.
struct HomeView: View {
    @ObservedObject var core: Core
    @State private var hasLoadedInitialData = false
    
    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 24) {
                    if case .home(let weatherData) = core.view.workflow {
                        if weatherData.main.temp.isNormal {
                            WeatherCard(weatherData: weatherData)
                        } else {
                            ProgressView("Loading weather data...")
                                .padding()
                        }
                    } else {
                        Text("Not in home view")
                            .foregroundColor(.secondary)
                    }
                    
                    Button(action: {
                        core.update(.home(.show(51.5074, -0.1278)))
                    }) {
                        HStack {
                            Image(systemName: "arrow.clockwise")
                            Text("Refresh Weather")
                        }
                        .padding(.horizontal, 20)
                        .padding(.vertical, 12)
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(12)
                    }
                    .padding(.bottom)
                }
                .padding(.vertical)
            }
            .background(Color(.systemGroupedBackground))
            .navigationTitle("Weather")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button(action: {
                        core.update(.navigate(Workflow.favorites(FavoritesState.idle)))
                    }) {
                        Image(systemName: "star")
                    }
                }
            }
            .onAppear {
                if !hasLoadedInitialData {
                    core.update(.home(.show(51.5074, -0.1278)))
                    hasLoadedInitialData = true
                }
            }
        }
    }
}

#Preview {
    HomeView(core: Core())
}
