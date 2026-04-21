import App
import SwiftUI

/// A sidebar row showing a location's weather summary.
struct LocationRow: View {
    let name: String
    let weather: FavoriteWeatherStateViewModel
    let isCurrentLocation: Bool

    var body: some View {
        HStack(spacing: 12) {
            weatherIcon

            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 4) {
                    if isCurrentLocation {
                        Image(systemName: "location.fill")
                            .font(.caption)
                    }
                    Text(name)
                        .font(.headline)
                }
                hiLoText
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            Spacer()

            temperatureText
                .font(.title2)
        }
    }

    @ViewBuilder
    private var weatherIcon: some View {
        Group {
            switch weather {
            case .fetching:
                ProgressView()
            case let .fetched(data):
                if let condition = data.weather.first {
                    WeatherIcon(weatherCode: condition.id, isDay: data.isDay, font: .title3)
                }
            case .failed:
                Image(systemName: "exclamationmark.triangle.fill")
                    .font(.title3)
                    .foregroundStyle(.secondary)
            }
        }
        .frame(width: 32, height: 32)
    }

    @ViewBuilder
    private var hiLoText: some View {
        switch weather {
        case .fetching:
            Text("Loading...")
        case let .fetched(data):
            Text("H:\(String(format: "%.0f°", data.main.tempMax))  L:\(String(format: "%.0f°", data.main.tempMin))")
        case .failed:
            Text("Unavailable")
        }
    }

    @ViewBuilder
    private var temperatureText: some View {
        switch weather {
        case .fetching:
            ProgressView()
        case let .fetched(data):
            Text(String(format: "%.0f°", data.main.temp))
        case .failed:
            Text("--")
                .foregroundStyle(.secondary)
        }
    }
}

/// Row for the current location, which has a different state model.
struct CurrentLocationRow: View {
    let localWeather: LocalWeatherViewModel

    var body: some View {
        switch localWeather {
        case .checkingPermission, .fetchingLocation:
            LocationRow(
                name: "My Location",
                weather: .fetching,
                isCurrentLocation: true
            )
        case .locationDisabled:
            LocationRow(
                name: "Location Disabled",
                weather: .failed,
                isCurrentLocation: true
            )
        case .fetchingWeather:
            LocationRow(
                name: "My Location",
                weather: .fetching,
                isCurrentLocation: true
            )
        case let .fetched(data):
            LocationRow(
                name: data.name,
                weather: .fetched(data),
                isCurrentLocation: true
            )
        case .failed:
            LocationRow(
                name: "My Location",
                weather: .failed,
                isCurrentLocation: true
            )
        }
    }
}

#Preview {
    List {
        CurrentLocationRow(localWeather: .fetched(previewWeatherResponse))
        LocationRow(
            name: "Paris",
            weather: .fetched(previewWeatherResponse),
            isCurrentLocation: false
        )
        LocationRow(
            name: "Tokyo",
            weather: .fetching,
            isCurrentLocation: false
        )
    }
}
