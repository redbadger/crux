import SwiftUI
import App
import os.log


/// A card view that displays the current weather information in a visually appealing way.
struct WeatherCard: View {
    let weatherData: CurrentResponse
    @Environment(\.colorScheme) var colorScheme
    
    private var isDay: Bool {
        let now = Date().timeIntervalSince1970
        return now >= Double(weatherData.sys.sunrise) && now <= Double(weatherData.sys.sunset)
    }
    
    private var gradientColors: [Color] {
        if let weather = weatherData.weather.first {
            switch weather.id {
            case 200...232: return [.gray, .blue] // Thunderstorm
            case 300...321: return [.blue, .gray] // Drizzle
            case 500...531: return [.blue, .gray] // Rain
            case 600...622: return [.white, .blue] // Snow
            case 701...781: return [.gray, .blue] // Atmosphere
            case 800: return isDay ? [.blue, .yellow] : [.blue, .purple] // Clear
            case 801...804: return [.gray, .blue] // Clouds
            default: return [.blue, .gray]
            }
        }
        return [.blue, .gray]
    }
    
    var body: some View {
        VStack(spacing: 16) {
            // Location
            VStack(spacing: 4) {
                Text(weatherData.name)
                    .font(.system(size: 28, weight: .bold))
            }
            
            // Temperature and Weather Icon
            VStack(spacing: 20) {
                if let weather = weatherData.weather.first {
                    WeatherIcon(weatherCode: weather.id, isDay: isDay)
                }
                
                HStack() {
                    Text(String(format: "%.1f°", weatherData.main.temp))
                        .font(.system(size: 64, weight: .medium))
                    
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
            }
            
            // Temperature Range
            HStack(spacing: 16) {
                HStack(spacing: 4) {
                    Image(systemName: "arrow.down")
                        .font(.caption)
                    Text(String(format: "%.1f°", weatherData.main.tempMin))
                }
                HStack(spacing: 4) {
                    Image(systemName: "arrow.up")
                        .font(.caption)
                    Text(String(format: "%.1f°", weatherData.main.tempMax))
                }
            }
            .font(.subheadline)
            .foregroundColor(.secondary)
            
            // Weather Details Grid
            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 16) {
                WeatherDetailItem(
                    icon: "thermometer",
                    title: "Feels Like",
                    value: String(format: "%.1f°", weatherData.main.feelsLike)
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
                WeatherDetailItem(
                    icon: "cloud",
                    title: "Clouds",
                    value: "\(weatherData.clouds.all)%"
                )
                WeatherDetailItem(
                    icon: "eye",
                    title: "Visibility",
                    value: "\(weatherData.visibility / 1000) km"
                )
            }
            .padding(.top, 8)
            
            // Sunrise and Sunset
            HStack(spacing: 20) {
                VStack(spacing: 4) {
                    Image(systemName: "sunrise.fill")
                        .font(.title2)
                        .foregroundColor(.orange)
                    TimeDisplay(timestamp: Int(weatherData.sys.sunrise))
                }
                
                VStack(spacing: 4) {
                    Image(systemName: "sunset.fill")
                        .font(.title2)
                        .foregroundColor(.orange)
                    TimeDisplay(timestamp: Int(weatherData.sys.sunset))
                }
            }
            .padding(.top, 8)
        }
        .padding(24)
        .background(
            RoundedRectangle(cornerRadius: 20)
                .fill(
                    LinearGradient(
                        gradient: Gradient(colors: gradientColors),
                        startPoint: .topLeading,
                        endPoint: .bottomTrailing
                    )
                )
                .opacity(colorScheme == .dark ? 0.3 : 0.1)
                .overlay(
                    RoundedRectangle(cornerRadius: 20)
                        .fill(Color(.systemBackground))
                        .opacity(0.9)
                )
        )
        .shadow(color: Color.black.opacity(0.1), radius: 10, x: 0, y: 5)
        .padding(.horizontal)
    }
} 
