import SwiftUI

/// A weather icon that adapts to the weather code and time of day.
struct WeatherIcon: View {
    let weatherCode: UInt64
    let isDay: Bool
    var font: Font = .title

    private var iconName: String {
        switch weatherCode {
        case 200...232: return "cloud.bolt.rain.fill"   // Thunderstorm
        case 300...321: return "cloud.drizzle.fill"     // Drizzle
        case 500...531: return "cloud.rain.fill"        // Rain
        case 600...622: return "cloud.snow.fill"        // Snow
        case 701...781: return "cloud.fog.fill"         // Atmosphere (fog, mist, etc)
        case 800: return isDay ? "sun.max.fill" : "moon.stars.fill" // Clear
        case 801: return "cloud.sun.fill"               // Few clouds
        case 802: return "cloud.fill"                   // Scattered clouds
        case 803...804: return "smoke.fill"             // Broken/Overcast clouds
        default: return "questionmark.circle.fill"
        }
    }

    var body: some View {
        Image(systemName: iconName)
            .font(font)
            .symbolRenderingMode(.multicolor)
    }
}

#Preview {
    WeatherIcon(weatherCode: 800, isDay: true)
}
