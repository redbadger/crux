import SwiftUI

class GetPlatform: Platform {
    func get() -> String {
        return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
    }
}

@MainActor
class Model: ObservableObject {
    @Published var fact = ""
    var core = Core()
    
    init() {
        self.update(msg: Msg.getFact)
    }
    
    private func getFact(url: String) {
        Task {
            let (data, _) = try! await URLSession.shared.data(from: URL(string: url)!)
            self.update(msg: .receiveFact(bytes: [UInt8](data)))
        }
    }
    
    func update(msg: Msg) {
        let cmd = core.update(msg)
        switch cmd {
        case .render(catFact: let catFact): self.fact = catFact
        case .get(url: let url): getFact(url: url)
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

struct ContentView: View {
    @ObservedObject var model: Model
    
    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(try! addForPlatform(1, 2, GetPlatform()))
            Text(model.fact).padding()
            HStack {
                ActionButton(label: "Clear", color: .red) {
                    model.update(msg: .clearFact)
                }
                ActionButton(label: "Get", color: .green) {
                    model.update(msg: .getFact)
                }
                ActionButton(label: "Fetch", color: .yellow) {
                    model.update(msg: .fetchFact)
                }
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Model())
    }
}
