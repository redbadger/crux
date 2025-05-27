import SwiftUI
import SharedTypes

struct HomeView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
        core.update(.home(HomeEvent.show(51.5074, -0.1278)))
        // Call Show event with some default coordinates (e.g., London)
        
    }
    
    var body: some View {
        VStack {
            if case .home(let weatherData) = core.view.workflow {
                if weatherData.main.temp.isNormal  {
                    Text("Weather Data:")
                        .font(.headline)
                    Text("Temperature: \(weatherData.main.temp)°C")
                    if let weather = weatherData.weather.first {
                        Text("Description: \(weather.description)")
                    }
                    Text("Location: \(weatherData.name)")
                } else {
                    Text("Loading weather data...")
                }
            } else {
                Text("Not in home view")
            }
            
            Button("Refresh Weather") {
                core.update(.home(.show(51.5074, -0.1278)))
            }
            .padding()
        }
        .padding()
    }
}

#Preview {
    HomeView(core: Core())
}
