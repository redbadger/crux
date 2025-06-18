import Foundation
import Shared
import SharedTypes

fileprivate class EffectHandler: CruxShell, @unchecked Sendable {
    public var handler: ((Data) -> Void)?

    func processEffects(_ bytes: Data) {
        // The handler is set straight after initialising the EffectHandler
        // it should never be nil
        handler?(bytes)
    }
}

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private var handler: EffectHandler
    private var core: CoreFfi

    init() {
        self.handler = EffectHandler()
        self.core = CoreFfi(handler)
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))

        // the handler assignment needs to be defered, otherwise we create a circular
        // reference between handler and self, before self is done initializing
        handler.handler = { bytes in
            let requests: [Request] = try! .bincodeDeserialize(input: [UInt8](bytes))
            for request in requests {
                self.processEffect(request)
            }
        }
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
            view = try! .bincodeDeserialize(input: [UInt8](core.view()))
        case let .http(req):
            Task {
                let response = try! await requestHttp(req).get()
                
                let effects = [UInt8](core.resolve(
                    request.id,
                    Data(try! HttpResult.ok(response).bincodeSerialize()))
                )
                
                let requests: [Request] = try! .bincodeDeserialize(input: effects)
                for request in requests {
                    processEffect(request)
                }
            }
        case let .serverSentEvents(req):
            Task {
                for await result in await requestSse(req) {
                    let response = try result.get()
                    
                    let effects = [UInt8](core.resolve(request.id, Data(try! response.bincodeSerialize())))

                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    for request in requests {
                        processEffect(request)
                    }
                }
            }
        }
    }
}
