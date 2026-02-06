import Foundation
import Shared

fileprivate class EffectHandler: NativeShell, @unchecked Sendable {
    var handler: ((NativeRequest) -> Void)?

    func handleEffect(_ request: NativeRequest) {
        handler?(request)
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
        self.view = core.view()

        handler.handler = { request in
            DispatchQueue.main.async {
                self.processEffect(request)
            }
        }
    }

    func update(_ event: EventFfi) {
        core.update(event)
    }

    func processEffect(_ request: NativeRequest) {
        switch request.effect {
        case .render:
            self.view = core.view()
        case let .http(req):
            Task {
                let result = await requestHttp(req)
                switch result {
                case .success(let response):
                    core.resolve(request.id, .http(.ok(response)))
                case .failure(let error):
                    core.resolve(request.id, .http(.err(.io("\(error)"))))
                }
            }
        case let .serverSentEvents(req):
            Task {
                for await result in await requestSse(req) {
                    switch result {
                    case .success(let response):
                        core.resolve(request.id, .serverSentEvents(response))
                    case .failure(let error):
                        print("SSE error: \(error)")
                    }
                }
            }
        }
    }
}
