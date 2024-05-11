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
                        Data(request.uuid),
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
            let response = TimeResponse.now(Instant(seconds: UInt64(now), nanos: 0))

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .platform:
            let response = PlatformResponse(value: get_platform())

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .keyValue(.get(key: _)):
            let response = KeyValueResponse.get(result: .data(value: []))

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .keyValue(.set(key: _, value: _)):
            let response = KeyValueResponse.set(result: .ok(previous: []))

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .keyValue(.delete(key: _)):
            let response = KeyValueResponse.delete(result: .ok(previous: []))

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        case .keyValue(.exists(key: _)):
            let response = KeyValueResponse.exists(result: .exists(value: false))

            let effects = [UInt8](handleResponse(Data(request.uuid), Data(try! response.bincodeSerialize())))

            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
        }
    }
}

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}
