import App
import UIKit
import Shared

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
        case .http(let req):
            Task {
                let response = try! await requestHttp(req).get()

                let effects = [UInt8](
                    core.resolve(
                        request.id,
                        Data(try! HttpResult.ok(response).bincodeSerialize())
                    )
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
                    
                    let effects = [UInt8](
                        core.resolve(
                            request.id,
                            Data(try! response.bincodeSerialize())
                        )
                    )
                    
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    for request in requests {
                        processEffect(request)
                    }
                }
            }
        }
    }
}

func get_platform() -> String {
    return UIDevice.current.systemName + " " + UIDevice.current.systemVersion
}
