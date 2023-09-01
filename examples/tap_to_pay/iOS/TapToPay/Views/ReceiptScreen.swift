import SwiftUI
import SharedTypes

struct ReceiptScreen: View {
    @Environment(\.update) var update

    var payment: Payment
    var receipt: Receipt

    private var email: Binding<String> {
        Binding (
            get: { return receipt.email },
            set: { email in
                if email != receipt.email {
                    update(.setReceiptEmail(email))
                }
            }
        )
    }

    var body: some View {
        VStack {
            Spacer()
            Image(systemName: "checkmark.seal.fill")
                .resizable()
                .foregroundColor(.accentColor)
                .aspectRatio(contentMode: .fit)
                .frame(width: 80)
                .padding()


            Text("Total").padding(.top)

            Text(String(format: "£%.2f", Double(payment.amount) / 100.0))
                .font(.system(size: 70.0))
                .fontDesign(.rounded)
                .lineLimit(1)
                .minimumScaleFactor(0.1)
                .padding(EdgeInsets(top: 0, leading: 50, bottom: 10, trailing: 50))

            HStack() {
                VStack { Divider() }
                Text("Email Receipt").lineLimit(1).padding().frame(width: 140)
                VStack { Divider() }
            }.padding(.horizontal)

            VStack() {
                HStack {
                    TextField("Enter email", text: email)
                        .keyboardType(.emailAddress)
                        .disabled(receipt.status != .new)
                        .padding()

                    if receipt.status == .pending {
                        ProgressView()
                            .padding()
                            .transition(.scale)
                    }

                    if receipt.status == .sent {
                        Image(systemName: "paperplane.circle.fill")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(height: 27)
                            .foregroundColor(.blue)
                            .padding(10.0)
                            .transition(.scale)
                    }
                }
                    .animation(
                        Animation.interpolatingSpring(mass: 0.2, stiffness: 25.0, damping: 3.0, initialVelocity: 17.0),
                        value: receipt.status
                    )
                    .background(Color.white)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8.0)
                            .stroke(.gray, lineWidth: 1)
                        )
                    .padding(.bottom)

                Button(action: { update(.sendReceipt) }) {
                    Text("Email my receipt")

                        .frame(maxWidth: .infinity)
                        .foregroundColor(.white)
                        .fontWeight(.bold)
                        .padding()
                        .background(receipt.status == .new ? Color.accentColor : Color.gray)
                        .cornerRadius(8.0)

                }
                    .disabled(receipt.status != .new)
            }
                .padding()
                .background(.white)
                .cornerRadius(20.0)
                .padding(.horizontal)


            Spacer()
            Button(action: {
                update(.completePayment)
            }) {
                Text("Done")
                    .frame(maxWidth: .infinity)
                    .foregroundColor(.white)
                    .fontWeight(.bold)
                    .font(.system(size: 18))
                    .padding(EdgeInsets(top: 20, leading: 60, bottom: 20, trailing: 60))
                    .background(Color.accentColor)
                    .cornerRadius(40.0)
                    .padding()
            }
        }.background(Color(hue: 0, saturation: 0, brightness: 0.95))
    }
}
struct ReceiptScreen_Previews: PreviewProvider {
    static var previews: some View {
        let newReceipt = Receipt(email: "", status: .new)
        let sentReceipt = Receipt(email: "bob@example.com", status: .sent)

        ReceiptScreen(payment: Payment(amount: 1500, status: .completed(newReceipt)), receipt: newReceipt)

        ReceiptScreen(payment: Payment(amount: 1500, status: .completed(sentReceipt)), receipt: sentReceipt)    }
}
