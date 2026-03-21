import SwiftUI

/// A weather icon that adapts to the weather code and time of day.
struct WeatherIcon: View {
    let weatherCode: UInt64
    let isDay: Bool
    
    private var iconName: String {
        switch weatherCode {
        case 200...232: return "cloud.bolt.rain" // Thunderstorm
        case 300...321: return "cloud.drizzle"   // Drizzle
        case 500...531: return "cloud.rain"      // Rain
        case 600...622: return "cloud.snow"      // Snow
        case 701...781: return "cloud.fog"       // Atmosphere (fog, mist, etc)
        case 800: return isDay ? "sun.max" : "moon.stars" // Clear
        case 801: return "cloud.sun"             // Few clouds
        case 802: return "cloud"                 // Scattered clouds
        case 803...804: return "smoke"           // Broken/Overcast clouds
        default: return "questionmark.circle"
        }
    }
    
    var body: some View {
        Image(systemName: iconName)
            .font(.system(size: 40))
            .symbolRenderingMode(.hierarchical)
            .foregroundStyle(.black)
            .shadow(color: Color.black.opacity(0.15), radius: 2, x: 0, y: 1)
    }
} 