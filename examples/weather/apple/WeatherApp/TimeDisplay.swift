import SwiftUI

/// A view that displays a formatted time from a timestamp.
struct TimeDisplay: View {
    let timestamp: Int
    
    var body: some View {
        Text(timestamp.formattedTime)
            .font(.caption)
            .foregroundColor(.secondary)
    }
}

private extension Int {
    var formattedTime: String {
        let date = Date(timeIntervalSince1970: TimeInterval(self))
        let formatter = DateFormatter()
        formatter.timeStyle = .short
        return formatter.string(from: date)
    }
} 