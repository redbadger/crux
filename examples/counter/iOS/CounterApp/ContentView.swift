import Serde
import SwiftUI

enum Outcome {
    case http(HttpResponse)
    case sse(SseResponse)
}

typealias Uuid = [UInt8]

enum Message {
    case event(Event)
    case response(Uuid, Outcome)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(text: "", confirmed: false)

    init() {
        update(msg: .event(.startWatch))
    }

    private func http(uuid: Uuid, method: String, url: String, headers: [HttpHeader]) {
        var req = URLRequest(url: URL(string: url)!)
        req.httpMethod = method

        for header in headers {
            req.addValue(header.value, forHTTPHeaderField: header.name)
        }

        Task {
            let (data, response) = try! await URLSession.shared.data(for: req)
            if let httpResponse = response as? HTTPURLResponse {
                let status = UInt16(httpResponse.statusCode)
                let body = [UInt8](data)
                self.update(msg: .response(uuid, .http(HttpResponse(status: status, body: body))))
            }
        }
    }

    private func sse(uuid: Uuid, url: String) {
        let req = URLRequest(url: URL(string: url)!)
        Task {
            let (asyncBytes, response) = try! await URLSession.shared.bytes(for: req)
            guard let httpResponse = response as? HTTPURLResponse,
                  (200 ... 299).contains(httpResponse.statusCode)
            else {
                // TODO: handle error
                return
            }

            for try await line in asyncBytes.lines {
                let line = line + "\n\n"
                self.update(msg: .response(uuid, .sse(.chunk([UInt8](line.utf8)))))
            }
        }
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case let .event(m):
            reqs = try! [Request].bcsDeserialize(input: CounterApp.processEvent(try! m.bcsSerialize()))
        case let .response(uuid, outcome):
            reqs = try! [Request].bcsDeserialize(input: CounterApp.handleResponse(uuid, {
                switch outcome {
                case let .http(x):
                    return try! x.bcsSerialize()
                case let .sse(x):
                    return try! x.bcsSerialize()
                }
            }()))
        }

        for req in reqs {
            switch req.effect {
            case .render: view = try! ViewModel.bcsDeserialize(input: CounterApp.view())
            case let .http(r): http(uuid: req.uuid, method: r.method, url: r.url, headers: r.headers)
            case let .serverSentEvents(r): sse(uuid: req.uuid, url: r.url)
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
            Text(String(model.view.text))
                .foregroundColor(model.view.confirmed ? Color.black : Color.gray)
                .padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    model.update(msg: .event(.decrement))
                }
                ActionButton(label: "Increment", color: .red) {
                    model.update(msg: .event(.increment))
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
