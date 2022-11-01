import SwiftUI

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}

enum Message {
    case message(Msg)
    case response(Response)
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
            self.update(msg: .response(.http(uuid: uuid, bytes: [UInt8](data))))
        }
    }

    func update(msg: Message) {
        let reqs: [Request]

        switch msg {
        case .message(let m):
            reqs = core.message(m)
        case .response(let r):
            reqs = core.response(r)
        }

        for req in reqs {
            switch req {
            case .render: view = core.view()
            case .http(url: let url, uuid: let uuid): httpGet(uuid: uuid, url: url)
            case .time(let uuid):
                update(msg: .response(.time(uuid: uuid, isoTime: Date().ISO8601Format())))
            case .platform(let uuid):
                update(msg: .response(.platform(uuid: uuid, platform: get_platform())))
            case .kvRead(uuid: let uuid, key: _):
                update(msg: .response(.kvRead(uuid: uuid, bytes: .none)))
            case .kvWrite(uuid: let uuid, key: _, bytes: _):
                update(msg: .response(.kvWrite(uuid: uuid, success: false)))
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
