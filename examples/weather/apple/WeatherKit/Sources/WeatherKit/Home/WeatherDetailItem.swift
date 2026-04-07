import SwiftUI

/// A reusable weather detail item for displaying a metric with an icon, title, and value.
struct WeatherDetailItem: View, Identifiable {
    var id: String { title }
    let icon: String
    let title: String
    let value: String

    var body: some View {
        GroupBox {
            HStack(spacing: 12) {
                Image(systemName: icon)
                    .font(.title2)
                    .foregroundStyle(.tint)

                VStack(alignment: .leading, spacing: 2) {
                    Text(title)
                        .font(.caption)
                        .foregroundStyle(.secondary)
                    Text(value)
                        .font(.system(.body, design: .rounded))
                        .fontWeight(.medium)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}

#Preview {
    WeatherDetailItem(icon: "thermometer", title: "Feels Like", value: "17.2°")
}
