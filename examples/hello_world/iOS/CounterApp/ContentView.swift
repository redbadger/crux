import SharedTypes
import SwiftUI

enum Message {
    case message(Event)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(count: "")

    init() {
        update(msg: .message(.reset))
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case let .message(m):
            reqs = try! [Request].bincodeDeserialize(input: CounterApp.processEvent(try! m.bincodeSerialize()))
        }

        for req in reqs {
            switch req.effect {
            case .render(_): view = try! ViewModel.bincodeDeserialize(input: CounterApp.view())
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
                    model.update(msg: .message(.reset))
                }
                ActionButton(label: "Inc", color: .green) {
                    model.update(msg: .message(.increment))
                }
                ActionButton(label: "Dec", color: .yellow) {
                    model.update(msg: .message(.decrement))
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
