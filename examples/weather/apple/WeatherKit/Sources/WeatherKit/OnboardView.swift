import App
import SwiftUI

public struct OnboardView: View {
    @Environment(CoreUpdater.self) var update
    let model: OnboardViewModel

    public init(model: OnboardViewModel) {
        self.model = model
    }

    public var body: some View {
        VStack(spacing: 24) {
            Text(reasonMessage)
                .font(.headline)
                .multilineTextAlignment(.center)

            switch model.state {
            case let .input(apiKey, canSubmit):
                VStack(spacing: 16) {
                    HStack {
                        Image(systemName: "key")
                            .foregroundColor(.secondary)
                        TextField("API Key", text: Binding(
                            get: { apiKey },
                            set: { update(.onboard(.apiKey($0))) }
                        ))
                        .textFieldStyle(RoundedBorderTextFieldStyle())
                    }

                    HStack {
                        Spacer()
                        Button("Submit") {
                            update(.onboard(.submit))
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.extraLarge)
                        .disabled(!canSubmit)
                    }
                }

            case .saving:
                ProgressView("Saving...")
            }
        }
        .padding()
    }

    private var reasonMessage: String {
        switch model.reason {
        case .welcome:
            "Enter your OpenWeather API key"
        case .unauthorized:
            "API key was rejected. Please try again."
        case .reset:
            "Enter a new API key"
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
