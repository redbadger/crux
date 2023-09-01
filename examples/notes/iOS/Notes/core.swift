import Foundation
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    
    init() {
        self.view = try! .bincodeDeserialize(input: [UInt8](Notes.view()))
    }

    func update(_ event: Event) {
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))

        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            view = try! .bincodeDeserialize(input: [UInt8](Notes.view()))
        case let .pubSub(.publish(bytes)):
            print(["Publish", bytes.count, "bytes"] as [Any])
        case .pubSub(.subscribe):
            print("Subscribe")
        case .keyValue(_): ()
        case .timer(_): ()
        }
    }
}
