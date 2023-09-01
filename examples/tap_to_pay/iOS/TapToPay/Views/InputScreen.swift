import SwiftUI

struct InputScreen: View {
    @Environment(\.update) var update
    var amount = 0

    var body: some View {
        VStack {
            Spacer()
            AmountText(amount: amount)
            Spacer()
            Divider()
            Grid(horizontalSpacing: 0.0, verticalSpacing: 0.0) {
                GridRow {
                    KeyboardButton(action: { insertNumber(number: 1) }) {
                        Text("1").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 2) }) {
                        Text("2").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 3) }) {
                        Text("3").font(.system(size: 50.0))
                    }
                }
                GridRow {
                    KeyboardButton(action: { insertNumber(number: 4) }) {
                        Text("4").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 5) }) {
                        Text("5").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 6) }) {
                        Text("6").font(.system(size: 50.0))
                    }
                }
                GridRow {
                    KeyboardButton(action: { insertNumber(number: 7) }) {
                        Text("7").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 8) }) {
                        Text("8").font(.system(size: 50.0))
                    }
                    KeyboardButton(action: { insertNumber(number: 9) }) {
                        Text("9").font(.system(size: 50.0))
                    }
                }
                GridRow {
                    KeyboardButton(action: { removeNumber() } ) {
                        Image(systemName: "delete.backward.fill")
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(width: 40.0, height: 40.0)
                    }
                    KeyboardButton(action: { insertNumber(number: 0) }) {
                        Text("0").font(.system(size: 50.0))
                    }
                }

            }
            PayButton() {
                update(.startPayment)
            }
        }
    }

    func insertNumber(number: UInt32) {
        update(.setAmount(UInt32(amount * 10) + number))
    }

    func removeNumber() {
        update(.setAmount(UInt32(amount / 10)))
    }
}

struct PayButton: View {
    var action: () -> Void

    init(action: @escaping () -> Void) {
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            HStack {
                Image(systemName: "wave.3.right.circle.fill").imageScale(.large)
                Spacer()
                Text("Tap to Pay on iPhone")
                Spacer()
            }
            .foregroundColor(.white)
            .fontWeight(.bold)
            .font(.system(size: 18))
            .padding(EdgeInsets(top: 20, leading: 20, bottom: 20, trailing: 20))
            .background(Color.accentColor)
            .cornerRadius(40.0)
        }
        .padding(.all)
    }
}

struct KeyboardButton<Content>: View where Content : View {
    var action: () -> Void
    var content: () -> Content

    init(action: @escaping () -> Void, content: @escaping () -> Content) {
        self.action = action
        self.content = content
    }

    var body: some View {
        Button(action: action) {
            ZStack(content: self.content)
                .foregroundColor(.primary)
        }
            .frame(width: 120, height: 90)
    }
}

struct AmountText: View {
    var amount = 0

    var body: some View {
        let amount = Double(amount) / 100.0

        Text(String(format: "£%.2f", amount))
            .font(.system(size: 70.0))
            .fontDesign(.rounded)
            .foregroundColor(.accentColor)
            .lineLimit(1)
            .minimumScaleFactor(0.1)
            .padding(.minimum(50.0, 50.0))
    }
}
