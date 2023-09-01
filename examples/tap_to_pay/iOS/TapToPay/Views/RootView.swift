import SwiftUI
import SharedTypes

struct RootView: View {
    @StateObject var core = Core()

    private var paymentInProgress: Binding<Bool> {
        Binding (
            get: {
                if case let .payment(p) = core.view.screen {
                    return p.status != .new
                }

                return false
            },
            set: { value in
                if !value {
                    self.core.update(.abortPayment)
                }
            }
        )
    }

    var body: some View {
        NavigationStack() {
            switch core.view.screen {
            case .payment(let payment):
                InputScreen(amount: Int(payment.amount))
                    .navigationTitle("Tap to Pay")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar() {
                        ToolbarItem(placement: .primaryAction) {
                            // FIXME this won't work long-term, it wants to manage its own
                            // navigation state
                            NavigationLink(destination: {
                                VStack {
                                    Text("To do: Settings")
                                }
                            }) {
                                Image(systemName: "gear")
                            }
                        }
                    }
                    .sheet(isPresented: paymentInProgress) {
                        PaymentFlow(payment: payment)
                    }
            }
        }
        .environment(\.update, { e in core.update(e)})
    }
}

private struct UpdateKey: EnvironmentKey {
    static let defaultValue: (Event) -> Void = { _ in }
}

extension EnvironmentValues {
  var update: (Event) -> Void {
    get { self[UpdateKey.self] }
    set { self[UpdateKey.self] = newValue }
  }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        RootView(core: Core())
    }
}
