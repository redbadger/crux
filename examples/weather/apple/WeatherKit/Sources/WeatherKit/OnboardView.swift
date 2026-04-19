import App
import SwiftUI

public struct OnboardView: View {
    @Environment(CoreUpdater.self) var update
    let model: OnboardViewModel

    public init(model: OnboardViewModel) {
        self.model = model
    }

    public var body: some View {
        VStack(spacing: 32) {
            Spacer()

            // Branding - since this is the first screen
            Image(systemName: "cloud.sun.fill")
                .font(.system(size: 72))
                .symbolRenderingMode(.multicolor)

            Text("Crux Weather")

                .font(.largeTitle)
                .fontWeight(.bold)

            Text(reasonMessage)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .multilineTextAlignment(.center)

            // Ask for the API key
            switch model.state {
            case let .input(apiKey, canSubmit):
                VStack(spacing: 16) {
                    TextField("Paste your API key here", text: Binding(
                        get: { apiKey },
                        set: { update(.onboard(.apiKey($0))) }
                    ))
                    .textFieldStyle(.roundedBorder)

                    Button("Get Started") {
                        update(.onboard(.submit))
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.extraLarge)
                    .disabled(!canSubmit)
                }
                .padding(.horizontal, 32)

            case .saving:
                ProgressView("Saving...")
            }

            Spacer()
            Spacer()
        }
        .padding()
    }

    private var reasonMessage: String {
        switch model.reason {
        case .welcome:
            "Enter your OpenWeather API key to get started."
        case .unauthorized:
            "That API key was rejected. Please try again."
        case .reset:
            "Enter a new API key."
        }
    }
}

#Preview {
    OnboardView(model: OnboardViewModel(
        reason: .welcome,
        state: .input(apiKey: "", canSubmit: false)
    ))
    .previewEnvironment()
}
