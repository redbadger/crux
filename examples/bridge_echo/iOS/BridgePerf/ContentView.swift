import SharedTypes
import SwiftUI

enum Message {
    case event(Event)
}

@MainActor
class Model: ObservableObject {
    @Published var view = ViewModel(count: 0)

    init() {
        Task.init {
            while(true) {
                update(msg: Message.event(.tick))
                await Task.yield()
            }
        }

        Task.init {
            while(true) {
                try await Task.sleep(nanoseconds: UInt64(Double(NSEC_PER_SEC)))
                update(msg: Message.event(.newPeriod))
            }
        }

        update(msg: Message.event(.tick))
    }

    func update(msg: Message) {
        let requests: [Request]

        switch msg {
        case let .event(event):
            requests = try! .bincodeDeserialize(
                input: [UInt8](processEvent(Data(try! event.bincodeSerialize())))
            )
        }

        for req in requests {
            switch req.effect {
            case .render:
                view = try! .bincodeDeserialize(input: [UInt8](BridgePerf.view()))
            }
        }
    }
}

struct ContentView: View {
    @ObservedObject var model: Model

    var body: some View {
        VStack {
            Text(String(model.view.count)).font(.largeTitle)
        }
    }
}

struct ContentView_Previews: PreviewProvider {
    static var previews: some View {
        ContentView(model: Model())
    }
}
