import SwiftUI

struct StatusCard: View {
    let message: String
    var icon: String = "hourglass"

    var body: some View {
        GroupBox {
            VStack(spacing: 20) {
                Image(systemName: icon)
                    .font(.largeTitle)
                    .foregroundStyle(.secondary)
                Text(message)
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .padding()
    }
}

#Preview {
    StatusCard(message: "Checking location permission...")
}
