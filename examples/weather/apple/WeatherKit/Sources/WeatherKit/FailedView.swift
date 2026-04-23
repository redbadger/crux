import SwiftUI

public struct FailedView: View {
    let message: String

    public init(message: String) {
        self.message = message
    }

    public var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.largeTitle)
                .foregroundStyle(.red)
            Text(message)
                .foregroundStyle(.secondary)
        }
    }
}

#Preview {
    FailedView(message: "Something went wrong")
}
