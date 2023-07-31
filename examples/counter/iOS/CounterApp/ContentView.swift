import SharedTypes
import SwiftUI

@MainActor
class Core: ObservableObject {
    @Published var view = ViewModel(text: "", confirmed: false)

    init() {
        update(event: .startWatch)
    }

    func update(event: Event) {
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))

        process_effects(effects)
    }

    func process_effects(_ effects: [UInt8]) {
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            process_request(request)
        }
    }

    func process_request(_ request: Request) {
        switch request.effect {
        case .render:
            view = try! .bincodeDeserialize(input: [UInt8](CounterApp.view()))
        case let .http(req):
            Task {
                let response = try! await requestHttp(req).get()

                let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

                process_effects(effects)
            }
        case let .serverSentEvents(req):
            Task {
                for await result in await requestSse(req) {
                    let response = try! result.get()

                    let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

                    process_effects(effects)
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
    @ObservedObject var model: Core

    var body: some View {
        VStack {
            Text("Crux Counter Example").font(.headline)
            Text("Rust Core, Swift Shell (SwiftUI)").padding()
            Text(String(model.view.text))
                .foregroundColor(model.view.confirmed ? Color.black : Color.gray)
                .padding()
            HStack {
                ActionButton(label: "Decrement", color: .yellow) {
                    model.update(event: .decrement)
                }
                ActionButton(label: "Increment", color: .red) {
                    model.update(event: .increment)
                }
            }
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Core())
    }
}
