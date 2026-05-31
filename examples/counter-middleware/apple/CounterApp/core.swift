import App
import Foundation
import Shared

private class EffectHandler: CruxShell, @unchecked Sendable {
    public var handler: ((Data) -> Void)?

    func processEffects(bytes: Data) {
        handler?(bytes)
    }
}

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private var handler: EffectHandler
    private var core: CoreFFI

    init() {
        self.handler = EffectHandler()
        self.core = CoreFFI(shell: handler)
        // swiftlint:disable:next force_try
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))

        handler.handler = { bytes in
            // swiftlint:disable:next force_try
            let requests = try! Requests.bincodeDeserialize(input: [UInt8](bytes)).value
            for request in requests {
                self.processEffect(request)
            }
        }
    }

    func update(_ event: Event) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.update(data: Data(try! event.bincodeSerialize())))

        // swiftlint:disable:next force_try
        let requests = try! Requests.bincodeDeserialize(input: effects).value
        for request in requests {
            processEffect(request)
        }
    }

    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            DispatchQueue.main.async {
                // swiftlint:disable:next force_try
                self.view = try! .bincodeDeserialize(input: [UInt8](self.core.view()))
            }
        case .http(let httpRequest):
            Task {
                let result = await performHttpRequest(httpRequest)
                // swiftlint:disable force_try
                let effects = [UInt8](
                    self.core.resolve(
                        effectId: request.id,
                        data: Data(try! result.bincodeSerialize())
                    ))
                let requests = try! Requests.bincodeDeserialize(input: effects).value
                // swiftlint:enable force_try
                for request in requests {
                    self.processEffect(request)
                }
            }
        case .serverSentEvents(let sseRequest):
            Task {
                await performSseRequest(sseRequest, requestId: request.id)
            }
        case .random:
            // Handled internally by middleware, should not reach the shell
            fatalError("Unexpected Random effect in shell")
        }
    }

    func performHttpRequest(_ request: HttpRequest) async -> HttpResult {
        guard let url = URL(string: request.url) else {
            return .err(.url("Invalid URL"))
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = request.method
        for header in request.headers {
            urlRequest.addValue(header.value, forHTTPHeaderField: header.name)
        }

        do {
            let (data, response) = try await URLSession.shared.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return .err(.io("Not an HTTP response"))
            }
            let headers = (httpResponse.allHeaderFields as? [String: String] ?? [:])
                .map { HttpHeader(name: $0.key, value: $0.value) }
            return .ok(
                HttpResponse(
                    status: UInt16(httpResponse.statusCode),
                    headers: headers,
                    body: [UInt8](data)
                )
            )
        } catch {
            return .err(.io(error.localizedDescription))
        }
    }

    func performSseRequest(_ request: SseRequest, requestId: UInt32) async {
        guard let url = URL(string: request.url) else { return }

        do {
            let (bytes, _) = try await URLSession.shared.bytes(from: url)
            var buffer = Data()

            for try await byte in bytes {
                buffer.append(byte)
                if buffer.suffix(2) == Data([0x0A, 0x0A]) {
                    let response = SseResponse.chunk([UInt8](buffer))
                    buffer = Data()
                    // swiftlint:disable force_try
                    let effects = [UInt8](
                        self.core.resolve(
                            effectId: requestId,
                            data: Data(try! response.bincodeSerialize())
                        ))
                    let requests = try! Requests.bincodeDeserialize(input: effects).value
                    // swiftlint:enable force_try
                    for request in requests {
                        self.processEffect(request)
                    }
                }
            }

            let done = SseResponse.done
            // swiftlint:disable force_try
            let effects = [UInt8](
                self.core.resolve(
                    effectId: requestId,
                    data: Data(try! done.bincodeSerialize())
                ))
            let requests = try! Requests.bincodeDeserialize(input: effects).value
            // swiftlint:enable force_try
            for request in requests {
                self.processEffect(request)
            }
        } catch {
            print("SSE error: \(error)")
        }
    }
}
