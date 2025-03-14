import Foundation
import SharedTypes
import UIKit

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    init() {
        view = try! .bincodeDeserialize(input: [UInt8](CatFacts.view()))
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
            view = try! .bincodeDeserialize(input: [UInt8](CatFacts.view()))
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
        case .time:
            let now = Date().timeIntervalSince1970;
            let response = TimeResponse.now(instant: Instant(seconds: UInt64(now), nanos: 0))
            
            let effects = [UInt8](handleResponse(request.id, Data(try! response.bincodeSerialize())))
            
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .platform:
            let response = PlatformResponse(value: get_platform())
            
            let effects = [UInt8](handleResponse(request.id, Data(try! response.bincodeSerialize())))
            
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .keyValue: ()
        }
    }
}

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}
