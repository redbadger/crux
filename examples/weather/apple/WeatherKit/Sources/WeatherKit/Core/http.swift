import App
import Foundation

private nonisolated let logger = Log.http

/// The app's `HttpHandler`, over a `URLSession`.
///
/// Every response is an answer, whatever its status. Only a transport failure
/// is an error, as the protocol says.
nonisolated final class SessionHttpHandler: HttpHandler {
    private let session: URLSession

    init(session: URLSession = .shared) {
        self.session = session
    }

    func request(_ operation: HttpRequest) async -> HttpResult {
        logger.debug("sending \(operation.method) \(operation.url)")

        // `URL(string:)` also accepts a relative URL, so check for a scheme.
        guard let url = URL(string: operation.url), url.scheme != nil else {
            return .err(.url("Not an absolute URL: \(operation.url)"))
        }

        let method = operation.method.uppercased()
        if !operation.body.isEmpty, method == "GET" || method == "HEAD" {
            return .err(.io("A \(method) request cannot have a body"))
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = operation.method
        for header in operation.headers {
            urlRequest.addValue(header.value, forHTTPHeaderField: header.name)
        }
        if !operation.body.isEmpty {
            urlRequest.httpBody = Data(operation.body)
        }

        do {
            let (data, response) = try await session.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return .err(.io("Not an HTTP response"))
            }
            logger.debug("received \(httpResponse.statusCode) from \(operation.url)")
            let headers = httpResponse.allHeaderFields.compactMap { name, value in
                (name as? String).map { HttpHeader(name: $0, value: "\(value)") }
            }
            return .ok(
                HttpResponse(
                    status: UInt16(httpResponse.statusCode),
                    headers: headers,
                    body: [UInt8](data)
                )
            )
        } catch let error as URLError where error.code == .timedOut {
            logger.debug("request timed out: \(operation.url)")
            return .err(.timeout)
        } catch let error as URLError where error.code == .badURL || error.code == .unsupportedURL {
            logger.warning("bad URL: \(operation.url)")
            return .err(.url(error.localizedDescription))
        } catch {
            logger.warning("request failed: \(error.localizedDescription)")
            return .err(.io(error.localizedDescription))
        }
    }
}
