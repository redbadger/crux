import SharedTypes
import SwiftUI

enum Message {
    case event(Event)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(count: "")

    init() {
        update(msg: .event(.reset))
    }

    func update(msg: Message) {
        let requests: [Request]

        switch msg {
        case let .event(event):
            requests = try! .bincodeDeserialize(
                input: processEvent(try! event.bincodeSerialize())
            )
        }

        for req in requests {
            switch req.effect {
            case .render:
                view = try! .bincodeDeserialize(input: CounterApp.view())
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

struct ContentView: View {
    @ObservedObject var model: Model

    var body: some View {
        VStack {
            Image(systemName: "globe")
                .imageScale(.large)
                .foregroundColor(.accentColor)
            Text(model.view.count)
            HStack {
                ActionButton(label: "Reset", color: .red) {
                    model.update(msg: .event(.reset))
                }
                ActionButton(label: "Inc", color: .green) {
                    model.update(msg: .event(.increment))
                }
                ActionButton(label: "Dec", color: .yellow) {
                    model.update(msg: .event(.decrement))
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
