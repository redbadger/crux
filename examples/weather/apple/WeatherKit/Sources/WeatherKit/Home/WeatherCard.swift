import App
import SwiftUI

/// A card view that displays the current weather information.
struct WeatherCard: View {
    let weatherData: CurrentWeatherResponse

    private var detailItems: [WeatherDetailItem] {
        [
            WeatherDetailItem(
                icon: "thermometer",
                title: "Feels Like",
                value: String(format: "%.1f°", weatherData.main.feelsLike)
            ),
            WeatherDetailItem(
                icon: "humidity",
                title: "Humidity",
                value: "\(weatherData.main.humidity)%"
            ),
            WeatherDetailItem(
                icon: "wind",
                title: "Wind",
                value: String(format: "%.1f m/s", weatherData.wind.speed)
            ),
            WeatherDetailItem(
                icon: "gauge",
                title: "Pressure",
                value: "\(weatherData.main.pressure) hPa"
            ),
            WeatherDetailItem(
                icon: "cloud",
                title: "Clouds",
                value: "\(weatherData.clouds.all)%"
            ),
            WeatherDetailItem(
                icon: "eye",
                title: "Visibility",
                value: "\(weatherData.visibility / 1000) km"
            ),
        ]
    }

    var body: some View {
        GroupBox {
            VStack(spacing: 16) {
                // Weather Icon
                if let weather = weatherData.weather.first {
                    WeatherIcon(weatherCode: weather.id, isDay: weatherData.isDay, font: .largeTitle)
                }

                // Location
                Text(weatherData.name)
                    .font(.headline)

                // Temperature and condition
                HStack(alignment: .firstTextBaseline) {
                    Text(String(format: "%.0f°", weatherData.main.temp))
                        .font(.largeTitle)

                    if let weather = weatherData.weather.first {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(weather.main)
                                .font(.title3)
                                .fontWeight(.semibold)
                            Text(weather.description.capitalized)
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
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
                .foregroundStyle(.secondary)

                // Weather Details - 2x3 grid when wide, single column when narrow
                ViewThatFits(in: .horizontal) {
                    LazyVGrid(columns: [
                        GridItem(.flexible()),
                        GridItem(.flexible())
                    ], spacing: 12) {
                        ForEach(detailItems) { $0 }
                    }

                    VStack(spacing: 12) {
                        ForEach(detailItems) { $0 }
                    }
                }

                // Sunrise and Sunset
                HStack(spacing: 20) {
                    VStack(spacing: 4) {
                        Image(systemName: "sunrise.fill")
                            .font(.title2)
                            .foregroundStyle(.orange)
                        TimeDisplay(timestamp: Int(weatherData.sys.sunrise))
                    }

                    VStack(spacing: 4) {
                        Image(systemName: "sunset.fill")
                            .font(.title2)
                            .foregroundStyle(.orange)
                        TimeDisplay(timestamp: Int(weatherData.sys.sunset))
                    }
                }
            }
        }
        .frame(minWidth: 300)
        .padding(.horizontal)
    }
}

#Preview {
    WeatherCard(weatherData: previewWeatherResponse)
}
