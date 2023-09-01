import SwiftUI
import SharedTypes

struct PaymentFlow: View {
    @Environment(\.update) var update
    var payment: Payment

    private var path: Binding<[Receipt]> {
        return Binding(
            get: {
                switch payment.status {
                case .completed(let receipt): return [receipt]
                default: return []
                }
            },
            set: { _ in }
        )
    }

    private var showTapToPay: Binding<Bool> {
        Binding(get: { payment.status == .pendingTap }, set: { _ in })
    }

    var body: some View {
        NavigationStack(path: path) {
            VStack {
                Text(payment.status == .pendingTap ? "Please wait" : "Processing your payment...")
                    .padding()

                ProgressView()
            }
            .navigationDestination(for: Receipt.self) { receipt in
                ReceiptScreen(payment: payment, receipt: receipt)
                    .navigationTitle("Receipt")
                    .navigationBarBackButtonHidden(true)
            }
        }
        .fullScreenCover(isPresented: showTapToPay) {
            TapPlaceholder(payment: payment)
                .navigationBarBackButtonHidden(true)
        }
    }
}

// This is temporary, in the final version, this will be
// presented by the tap to pay capability
struct TapPlaceholder: View {
    @Environment(\.update) var update
    var payment: Payment

    var body: some View {
        VStack {
            Spacer()
            Image(systemName: "wave.3.right.circle")
                .resizable()
                .aspectRatio(contentMode: .fit)
                .frame(width: 80, height: 80)
            Spacer()
            VStack {
                Text(String(format: "£%.2f", Double(payment.amount) / 100.0))
                    .font(.system(size: 70.0))
                    .fontDesign(.rounded)
                    .lineLimit(1)
                    .minimumScaleFactor(0.1)
                    .padding(.minimum(50.0, 20.0))
                    .frame(maxWidth: .infinity)

                Text("(this is a placeholder)")
                    .font(.system(size: 18.0))
                    .foregroundColor(Color.white)

                Button(action: {
                    update(.sendPayment)
                }) {
                    Text("Done")
                        .font(.system(size: 25.0))
                        .foregroundColor(.green)
                        .padding()
                }
            }
                .background(Color(hue: 0, saturation: 0, brightness: 0.4))
                .cornerRadius(35.0)
                .padding()

            Spacer(minLength: 200)
            Button(action: {
                update(.abortPayment)
            }) {
                Image(systemName: "xmark.circle.fill")
                    .resizable()
                    .foregroundColor(.white)
                    .aspectRatio(contentMode: .fit)
                    .frame(width: 70)
                    .padding(.all)
            }
        }
        .frame(minWidth: 0.0, maxWidth: .infinity)
        .background(.gray.gradient)
        .foregroundColor(.white)
    }
}


struct PaymentFlow_Previews: PreviewProvider {
    static var previews: some View {
        let payment = Payment(amount: 1500, status: .pendingTap)

        PaymentFlow(payment: payment)
    }
}
