import SwiftUI
import SharedTypes

struct WeatherCard: View {
    let weatherData: CurrentResponse
    
    var body: some View {
        VStack(spacing: 16) {
            // Location and Time
            VStack(spacing: 4) {
                Text(weatherData.name)
                    .font(.system(size: 28, weight: .bold))
                Text("Updated just now")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
            
            // Temperature and Weather Icon
            HStack(spacing: 20) {
                Text(String(format: "%.1f°", weatherData.main.temp))
                    .font(.system(size: 72, weight: .medium))
                
                if let weather = weatherData.weather.first {
                    VStack(alignment: .leading, spacing: 4) {
                        Text(weather.main)
                            .font(.title2)
                            .fontWeight(.semibold)
                        Text(weather.description.capitalized)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
            }
            
            // Weather Details Grid
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 16) {
                WeatherDetailItem(
                    icon: "thermometer",
                    title: "Feels Like",
                    value: String(format: "%.1f°", weatherData.main.feels_like)
                )
                WeatherDetailItem(
                    icon: "humidity",
                    title: "Humidity",
                    value: "\(weatherData.main.humidity)%"
                )
                WeatherDetailItem(
                    icon: "wind",
                    title: "Wind",
                    value: String(format: "%.1f m/s", weatherData.wind.speed)
                )
                WeatherDetailItem(
                    icon: "gauge",
                    title: "Pressure",
                    value: "\(weatherData.main.pressure) hPa"
                )
            }
            .padding(.top, 8)
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 20)
                .fill(Color(.systemBackground))
                .shadow(color: Color.black.opacity(0.1), radius: 10, x: 0, y: 5)
        )
        .padding(.horizontal)
    }
}

struct WeatherDetailItem: View {
    let icon: String
    let title: String
    let value: String
    
    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: icon)
                .font(.system(size: 20))
                .foregroundColor(.blue)
                .frame(width: 24)
            
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(value)
                    .font(.system(.body, design: .rounded))
                    .fontWeight(.medium)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color(.secondarySystemBackground))
        .cornerRadius(12)
    }
}

struct HomeView: View {
    @ObservedObject var core: Core
    
    init(core: Core) {
        self.core = core
        core.update(.home(HomeEvent.show(51.5074, -0.1278)))
    }
    
    var body: some View {
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
    }
}

#Preview {
    HomeView(core: Core())
}
