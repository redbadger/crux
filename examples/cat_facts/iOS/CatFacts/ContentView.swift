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

    private func httpGet(uuid: [UInt8], url: String) {
        Task {
            let (data, _) = try! await URLSession.shared.data(from: URL(string: url)!)
            self.update(msg: .response(uuid, .http(HttpResponse(status: 200, body: [UInt8](data)))))
        }
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case let .event(m):
            reqs = try! [Request].bincodeDeserialize(input: CatFacts.processEvent(try! m.bincodeSerialize()))
        case let .response(uuid, outcome):
            reqs = try! [Request].bincodeDeserialize(input: CatFacts.handleResponse(uuid, { switch outcome {
            case let .platform(x):
                return try! x.bincodeSerialize()
            case let .time(x):
                return try! x.bincodeSerialize()
            case let .http(x):
                return try! x.bincodeSerialize()
            case let .key_value(x):
                return try! x.bincodeSerialize()
            }}()))
        }

        for req in reqs {
            switch req.effect {
            case .render: view = try! ViewModel.bincodeDeserialize(input: CatFacts.view())
            case let .http(r): httpGet(uuid: req.uuid, url: r.url)
            case .time:
                update(msg: .response(req.uuid, .time(TimeResponse(value: Date().ISO8601Format()))))
            case .platform:
                update(msg: .response(req.uuid, .platform(PlatformResponse(value: get_platform()))))
            case .keyValue(.read):
                update(msg: .response(req.uuid, .key_value(KeyValueOutput.read(.none))))
            case .keyValue(.write):
                update(msg: .response(req.uuid, .key_value(KeyValueOutput.write(false))))
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
