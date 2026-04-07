import App
import Foundation

private let logger = Log.http

extension Core {
    func resolveHttp(request: HttpRequest, requestId: UInt32) {
        Task {
            logger.debug("sending \(request.method) \(request.url)")
            let result = await performHttpRequest(request)
            resolve(requestId: requestId, serialize: { try result.bincodeSerialize() })
        }
    }

    private func performHttpRequest(_ request: HttpRequest) async -> HttpResult {
        guard let url = URL(string: request.url) else {
            return .err(.url("Invalid URL"))
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = request.method
        for header in request.headers {
            urlRequest.addValue(header.value, forHTTPHeaderField: header.name)
        }
        if !request.body.isEmpty {
            urlRequest.httpBody = Data(request.body)
        }

        do {
            let (data, response) = try await URLSession.shared.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return .err(.io("Not an HTTP response"))
            }
            logger.debug("received \(httpResponse.statusCode) from \(request.url)")
            let headers = (httpResponse.allHeaderFields as? [String: String] ?? [:])
                .map { HttpHeader(name: $0.key, value: $0.value) }
            return .ok(
                HttpResponse(
                    status: UInt16(httpResponse.statusCode),
                    headers: headers,
                    body: [UInt8](data)
                )
            )
        } catch let error as URLError where error.code == .timedOut {
            logger.debug("request timed out: \(request.url)")
            return .err(.timeout)
        } catch {
            logger.warning("request failed: \(error.localizedDescription)")
            return .err(.io(error.localizedDescription))
        }
    }
}
