import App
import SwiftUI

#if DEBUG

// MARK: - Sample Weather Response

let previewWeatherResponse = CurrentWeatherResponse(
    coord: Coord(lat: 51.5074, lon: -0.1278),
    weather: [WeatherData(id: 800, main: "Clear", description: "clear sky", icon: "01d")],
    base: "stations",
    main: Main(temp: 18.5, feelsLike: 17.2, tempMin: 16.0, tempMax: 21.0, pressure: 1013, humidity: 65),
    visibility: 10000,
    wind: Wind(speed: 3.5, deg: 220, gust: nil),
    clouds: Clouds(all: 5),
    dt: UInt64(Date().timeIntervalSince1970),
    sys: Sys(
        type: 2, id: 1, country: "GB",
        sunrise: UInt64(Date().timeIntervalSince1970 - 21600),
        sunset: UInt64(Date().timeIntervalSince1970 + 21600)
    ),
    timezone: 0,
    id: 2643743,
    name: "London",
    cod: 200
)

// MARK: - Preview Environment Modifier

struct PreviewEnvironment: ViewModifier {
    func body(content: Content) -> some View {
        content
            .environment(Core.forPreviewing())
            .environment(CoreUpdater.forPreview())
    }
}

extension View {
    func previewEnvironment() -> some View {
        modifier(PreviewEnvironment())
    }
}

#endif
