import SwiftUI

struct LoadingCard: View {
    var body: some View {
        GroupBox {
            VStack(spacing: 20) {
                ProgressView()
                Text("Loading weather data...")
                    .foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .padding()
    }
}

#Preview {
    LoadingCard()
}
