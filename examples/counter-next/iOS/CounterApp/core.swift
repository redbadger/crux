import Foundation
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel
    
    init() {
        self.view = try! .bincodeDeserialize(input: [UInt8](CounterApp.view()))
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
            view = try! .bincodeDeserialize(input: [UInt8](CounterApp.view()))
        case let .http(req):
            Task {
                let response = try! await requestHttp(req).get()
                
                let effects = [UInt8](handleResponse(
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
                    
                    let effects = [UInt8](handleResponse(request.id, Data(try! response.bincodeSerialize())))
                    
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    for request in requests {
                        processEffect(request)
                    }
                }
            }
        }
    }
}
