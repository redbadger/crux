import Foundation
import SharedTypes
import UIKit


@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    init() {
        self.view = try! .bincodeDeserialize(input: [UInt8](Weather.view()))
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
            self.view = try! .bincodeDeserialize(input: [UInt8](Weather.view()))
        case let .http(req):
            Task {
                let response = try! await requestHttp(req).get()
                
                let effects = [UInt8](
                    handleResponse(
                        request.id,
                        Data(try! HttpResult.ok(response).bincodeSerialize())
                    )
                )
                
                let requests: [Request] = try! .bincodeDeserialize(input: effects)
                for request in requests {
                    processEffect(request)
                }
            }
            
        case .keyValue: ()
        }
    }
}
