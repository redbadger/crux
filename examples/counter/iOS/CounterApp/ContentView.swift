import Serde
import SwiftUI

enum Outcome {
    case http(HttpResponse)
}

enum Message {
    case message(Event)
    case response([UInt8], Outcome)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(text: "")

    init() {
        update(msg: .message(.get))
    }

    private func httpRequest(uuid: [UInt8], method: String, url: String) {
        var req = URLRequest(url: URL(string: url)!)
        req.httpMethod = method
        Task {
            let (data, _) = try! await URLSession.shared.data(for: req)
            self.update(msg: .response(uuid, .http(HttpResponse(status: 200, body: [UInt8](data)))))
        }
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case let .message(m):
            reqs = try! [Request].bcsDeserialize(input: CounterApp.message(try! m.bcsSerialize()))
        case let .response(uuid, outcome):
            reqs = try! [Request].bcsDeserialize(input: CounterApp.response(uuid, { switch outcome {
            case let .http(x):
                return try! x.bcsSerialize()
            }}()))
        }

        for req in reqs {
            switch req.effect {
            case .render(_): view = try! ViewModel.bcsDeserialize(input: CounterApp.view())
            case let .http(r): httpRequest(uuid: req.uuid, method: r.method, url: r.url)
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
            Text("Crux Counter Example").font(.headline)
            Text("Rust Core, Swift Shell (SwiftUI)").padding()
            Text(String(model.view.text)).padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    model.update(msg: .message(.decrement))
                }
                ActionButton(label: "Increment", color: .red) {
                    model.update(msg: .message(.increment))
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
