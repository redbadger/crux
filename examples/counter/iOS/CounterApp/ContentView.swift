import App
import SwiftUI

struct ContentView: View {
    @ObservedObject var core: Core

    init(core: Core) {
        self.core = core
        core.update(.startWatch)
    }

    var body: some View {
        VStack {
            Text("Crux Counter Example").font(.headline)
            Text("Rust Core, Swift Shell (SwiftUI)").padding()
            Text(String(core.view.text))
                .foregroundColor(core.view.confirmed ? Color.black : Color.gray)
                .padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    core.update(.decrement)
                }
                ActionButton(label: "Increment", color: .red) {
                    core.update(.increment)
                }
            }
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

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(core: Core())
    }
}
