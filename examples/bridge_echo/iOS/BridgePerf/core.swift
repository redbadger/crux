import Foundation
import Shared
import App

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private var core: CoreFfi

    init() {
        self.core = CoreFfi()
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
    }

    func update(_ event: Event) {
        let effects = [UInt8](core.update(Data(try! event.bincodeSerialize())))
        
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            DispatchQueue.main.async {
                self.view = try! .bincodeDeserialize(input: [UInt8](self.core.view()))
            }
        }
    }
}
