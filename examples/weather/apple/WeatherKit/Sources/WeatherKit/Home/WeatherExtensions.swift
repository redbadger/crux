import App
import Foundation

extension CurrentWeatherResponse {
    var isDay: Bool {
        let now = Date().timeIntervalSince1970
        return now >= Double(sys.sunrise) && now <= Double(sys.sunset)
    }
}
