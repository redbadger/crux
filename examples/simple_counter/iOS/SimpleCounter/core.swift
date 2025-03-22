import Foundation
import SharedTypes
import OSLog

extension Core: Shell {
    nonisolated
    func handleEffects(_ requests: Data) {
        let requests: [Request] = try! .bincodeDeserialize(input: [UInt8](requests))
        let logger = Logger(subsystem: "com.example.my-app", category: "app")
        logger.log("async requests received")
        dump(requests)
        Task {
            for request in requests {
                await processEffect(request)
            }
        }
    }
}


@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel!
    
    init() {
        // TODO: maybe unregister somewhere?
        new(self)
        self.view = try! .bincodeDeserialize(input: [UInt8](SimpleCounter.view()))
        self.update(.startInterval)
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
            view = try! .bincodeDeserialize(input: [UInt8](SimpleCounter.view()))
        }
    }
}
