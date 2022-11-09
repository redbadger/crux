import SwiftUI
import Serde

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}

enum Message {
    case message(Msg)
    case response(Response)
}

extension [Request] {
    public static func bcsDeserialize(input: [UInt8]) throws -> [Request] {
        let deserializer = BcsDeserializer.init(input: input)
        try deserializer.increase_container_depth()
        let length = try deserializer.deserialize_len()

        var requests: [Request] = []
        for _ in 0..<length {
            while deserializer.get_buffer_offset() < input.count {
                let req = try Request.deserialize(deserializer: deserializer)
                requests.append(req)
            }
        }
        deserializer.decrease_container_depth()

        return requests
    }
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
            self.update(msg: .response(Response(uuid: uuid, body: ResponseBody.http([UInt8](data)))))
        }
    }

    func update(msg: Message) {
        let reqs: [Request]
        
        switch msg {
        case .message(let m):
            reqs = try! [Request].bcsDeserialize(input: core.message(try! m.bcsSerialize()))
        case .response(let r):
            reqs = try! [Request].bcsDeserialize(input: core.response(try! r.bcsSerialize()))
        }

        for req in reqs {
            let uuid = req.uuid
            
            switch req.body {
            case .render: view = core.view()
            case .http(data: let data): httpGet(uuid: uuid, url: data)
            case .time:
                update(msg: .response(Response(uuid: uuid, body: ResponseBody.time(Date().ISO8601Format()))))
            case .platform:
                update(msg: .response(Response(uuid: uuid, body: ResponseBody.platform(get_platform()))))
            case .kVRead:
                update(msg: .response(Response(uuid: uuid, body: ResponseBody.kVRead(.none))))
            case .kVWrite:
                update(msg: .response(Response(uuid: uuid, body: ResponseBody.kVWrite(false))))
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
