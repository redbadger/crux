import SwiftUI
import Serde

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}

enum Message {
    case message(Msg)
    case response(Response)
}

func bincodeDeserialize(input: [UInt8]) throws -> Msg {
    let deserializer = BincodeDeserializer.init(input: input);
    let obj = try deserialize(deserializer: deserializer)
    if deserializer.get_buffer_offset() < input.count {
        throw DeserializationError.invalidInput(issue: "Some input bytes were not read")
    }
    return obj
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(fact: "", image: .none, platform: "")
    var core = Core()

    init() {
        update(msg: .message(.get))
        update(msg: .message(.getPlatform))
    }

    private func httpGet(uuid: [UInt8], url: String) {
        Task {
            let (data, _) = try! await URLSession.shared.data(from: URL(string: url)!)
            self.update(msg: .response(Response(uuid: uuid, body: .http([UInt8](data)))))
        }
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case .message(let m):
            reqs = reqs.bincodeDeserialize(core.message(try! m.bincodeSerialize()))
        case .response(let r):
            reqs = Request.bincodeDeserialize(core.response(try! r.bincodeSerialize()))
        }

        for req in reqs {
            switch req {
            case .render: view = core.view()
            case .http(data: let data): httpGet(uuid: data.uuid, url: data.body)
            case .time(let uuid):
                update(msg: .response(.time(data: StringEnvelope(body: Date().ISO8601Format(), uuid: uuid.uuid))))
            case .platform(let data):
                update(msg: .response(.platform(data: StringEnvelope(body: get_platform(), uuid: data.uuid))))
            case .kvRead(let data):
                update(msg: .response(.kvRead(data:OptionalBytesEnvelope(body: .none, uuid: data.uuid ))))
            case .kvWrite(let data):
                update(msg: .response(.kvWrite(data:BoolEnvelope(body: false, uuid: data.uuid))))
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
                    AsyncImage(url: URL(string: image.file)) { image in
                        image
                            .resizable()
                            .scaledToFit()
                    } placeholder: {
                        EmptyView()
                    }
                    .frame(maxHeight: 250)
                    .padding())
            } ?? AnyView(EmptyView())
            Text(model.view.fact).padding()
            HStack {
                ActionButton(label: "Clear", color: .red) {
                    model.update(msg: .message(.clear))
                }
                ActionButton(label: "Get", color: .green) {
                    model.update(msg: .message(.get))
                }
                ActionButton(label: "Fetch", color: .yellow) {
                    model.update(msg: .message(.fetch))
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
