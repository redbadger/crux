import SharedTypes
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

    func update(msg: Message) {
        var requests: [Request]

        switch msg {
        case let .event(event):
            requests = try! .bincodeDeserialize(
                input: [UInt8](processEvent(Data(try! event.bincodeSerialize())))
            )
        case let .response(uuid, .http(httpRes)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! httpRes.bincodeSerialize())))
            )
        case let .response(uuid, .sse(sseRes)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! sseRes.bincodeSerialize())))
            )
        }

        for request in requests {
            switch request.effect {
            case .render:
                view = try! .bincodeDeserialize(input: [UInt8](CounterApp.view()))
            case let .http(httpReq):
                Task {
                    let res = try! await httpRequest(httpReq).get()
                    update(msg: .response(request.uuid, .http(res)))
                }
            case let .serverSentEvents(sseReq):
                Task {
                    for await result in await sseRequest(sseReq) {
                        update(msg: .response(request.uuid, .sse(try! result.get())))
                    }
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
