import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    var body: some View {
        VStack {
            Text("Crux Counter Middleware Example")
                .font(.title)
                .padding()
            Text("Rust Core, Swift Shell (SwiftUI)")
                .padding(.bottom)
            Text(core.view.text)
                .foregroundColor(core.view.confirmed ? .primary : .secondary)
                .padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    core.update(.decrement)
                }
                ActionButton(label: "Increment", color: .red) {
                    core.update(.increment)
                }
            }
            ActionButton(label: "I'm feeling lucky", color: .purple) {
                core.update(.random)
            }
        }
        .onAppear {
            core.update(.startWatch)
        }
    }
}

struct ActionButton: View {
    var label: String
    var color: Color
    var action: () -> Void

    init(label: String, color: Color, action: @escaping () -> Void) {
        self.label = label
        self.color = color
        self.action = action
    }

    var body: some View {
        Button(action: action) {
            Text(label)
                .fontWeight(.bold)
                .font(.body)
                .padding(EdgeInsets(top: 10, leading: 15, bottom: 10, trailing: 15))
                .background(color)
                .cornerRadius(10)
                .foregroundColor(.white)
                .padding()
        }
    }
}

#Preview {
    ContentView(core: Core())
}
