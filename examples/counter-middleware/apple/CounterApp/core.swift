import App
import Foundation
import Shared
import ViewModel

@MainActor
class Core: ObservableObject {
    @Published var view: ViewModel

    private var core: CoreFfi

    init() {
        self.core = CoreFfi()
        // swiftlint:disable:next force_try
        self.view = try! .bincodeDeserialize(input: [UInt8](core.view()))
    }

    func update(_ event: Event) {
        // swiftlint:disable:next force_try
        let effects = [UInt8](core.update(Data(try! event.bincodeSerialize())))

        // swiftlint:disable:next force_try
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
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
        case let .http(httpRequest):
            Task {
                let result = await performHttpRequest(httpRequest)
                // swiftlint:disable:next force_try
                let effects = [UInt8](self.core.resolve(
                    request.id,
                    Data(try! result.bincodeSerialize())
                ))
                // swiftlint:disable:next force_try
                let requests: [Request] = try! .bincodeDeserialize(input: effects)
                for request in requests {
                    self.processEffect(request)
                }
            }
        case let .serverSentEvents(sseRequest):
            Task {
                await performSseRequest(sseRequest, requestId: request.id)
            }
        case let .random(randomRequest):
            let from = Int(randomRequest.field0)
            let to = Int(randomRequest.field1)
            let random = Int.random(in: from...to)
            let response = RandomNumber(field0: Int64(random))
            // swiftlint:disable:next force_try
            let effects = [UInt8](core.resolve(
                request.id,
                Data(try! response.bincodeSerialize())
            ))
            // swiftlint:disable:next force_try
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                processEffect(request)
            }
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

    func performSseRequest(_ request: ServerSentEvents.SseRequest, requestId: UInt32) async {
        guard let url = URL(string: request.url) else { return }

        do {
            let (bytes, _) = try await URLSession.shared.bytes(from: url)
            var buffer = Data()

            for try await byte in bytes {
                buffer.append(byte)
                if buffer.suffix(2) == Data([0x0A, 0x0A]) {
                    let response = ServerSentEvents.SseResponse.chunk([UInt8](buffer))
                    buffer = Data()
                    // swiftlint:disable:next force_try
                    let effects = [UInt8](self.core.resolve(
                        requestId,
                        Data(try! response.bincodeSerialize())
                    ))
                    // swiftlint:disable:next force_try
                    let requests: [Request] = try! .bincodeDeserialize(input: effects)
                    for request in requests {
                        self.processEffect(request)
                    }
                }
            }

            let done = ServerSentEvents.SseResponse.done
            // swiftlint:disable:next force_try
            let effects = [UInt8](self.core.resolve(
                requestId,
                Data(try! done.bincodeSerialize())
            ))
            // swiftlint:disable:next force_try
            let requests: [Request] = try! .bincodeDeserialize(input: effects)
            for request in requests {
                self.processEffect(request)
            }
        } catch {
            print("SSE error: \(error)")
        }
    }
}
