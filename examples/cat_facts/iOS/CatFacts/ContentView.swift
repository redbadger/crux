import SharedTypes
import SwiftUI

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}

enum Outcome {
    case platform(PlatformResponse)
    case time(TimeResponse)
    case http(HttpResponse)
    case key_value(KeyValueOutput)
}

enum Message {
    case event(Event)
    case response([UInt8], Outcome)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(fact: "", image: .none, platform: "")

    init() {
        update(msg: .event(.get))
        update(msg: .event(.getPlatform))
    }

    func update(msg: Message) {
        let requests: [Request]

        switch msg {
        case let .event(event):
            requests = try! .bincodeDeserialize(
                input: [UInt8](processEvent(Data(try! event.bincodeSerialize())))
            )
        case let .response(uuid, .http(response)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! response.bincodeSerialize())))
            )
        case let .response(uuid, .platform(response)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! response.bincodeSerialize())))
            )
        case let .response(uuid, .time(response)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! response.bincodeSerialize())))
            )
        case let .response(uuid, .key_value(response)):
            requests = try! .bincodeDeserialize(
                input: [UInt8](handleResponse(Data(uuid), Data(try! response.bincodeSerialize())))
            )
        }

        for request in requests {
            switch request.effect {
            case .render: view = try! ViewModel.bincodeDeserialize(input: [UInt8](CatFacts.view()))
            case let .http(httpReq):
                Task {
                    let res = try! await httpRequest(httpReq).get()
                    update(msg: .response(request.uuid, .http(res)))
                }
            case .time:
                update(msg: .response(request.uuid, .time(TimeResponse(value: Date().ISO8601Format()))))
            case .platform:
                update(msg: .response(request.uuid, .platform(PlatformResponse(value: get_platform()))))
            case .keyValue(.read):
                update(msg: .response(request.uuid, .key_value(KeyValueOutput.read(.none))))
            case .keyValue(.write):
                update(msg: .response(request.uuid, .key_value(KeyValueOutput.write(false))))
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
            Text(model.view.platform)
            model.view.image.map { image in
                AnyView(
                    // For the loading image to work properly, we'd need to add
                    // caching here
                    AsyncImage(url: URL(string: image.href)) { image in
                        image
                            .resizable()
                            .scaledToFit()
                    } placeholder: {
                        EmptyView()
                    }
                    .frame(maxHeight: 250)
                    .padding()
                )
            } ?? AnyView(EmptyView())
            Text(model.view.fact).padding()
            HStack {
                ActionButton(label: "Clear", color: .red) {
                    model.update(msg: .event(.clear))
                }
                ActionButton(label: "Get", color: .green) {
                    model.update(msg: .event(.get))
                }
                ActionButton(label: "Fetch", color: .yellow) {
                    model.update(msg: .event(.fetch))
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
