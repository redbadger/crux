import Foundation
#if canImport(FoundationNetworking)
    // On corelibs-foundation, URLSession and its companions live in a module
    // of their own; on Apple platforms `Foundation` already has them.
    import FoundationNetworking
#endif

/// The shell side of `crux_http`, as one method per operation.
///
/// The method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let http = URLSessionHttpHandler.shared
///
///     func http(_ operation: HttpRequest) async -> HttpResult {
///         await http.request(operation)
///     }
/// }
/// ```
///
/// Conform your own type to it to replace the implementation wholesale, in the
/// app or in a test.
public protocol HttpHandler: Sendable {
    /// Perform `operation` and answer with its result, exactly once.
    func request(_ operation: HttpRequest) async -> HttpResult
}

/// An `HttpHandler` over `URLSession`.
///
/// ## What is a failure
///
/// Every exchange the session completes is an `HttpResult.ok`, whatever the
/// status: 4xx and 5xx are answers, and the core decides what to make of them.
/// Only a transport failure — the request never reached a server, or no
/// response came back — is an `HttpResult.err`, and then:
///
/// - a URL `URL(string:)` will not parse, and `URLError.badURL` or
///   `.unsupportedURL`, are `HttpError.url`;
/// - `URLError.timedOut` is `HttpError.timeout`;
/// - everything else is `HttpError.io`, carrying the error's description.
public final class URLSessionHttpHandler: HttpHandler {
    /// The plain path: one handler over `URLSession.shared`.
    public static let shared = URLSessionHttpHandler()

    private let session: URLSession

    /// Performs every request on `session`.
    ///
    /// A pinned session, a session with a proxy configuration, or one with a
    /// delegate of your own is the shell's business, and this is where it goes.
    public init(session: URLSession = .shared) {
        self.session = session
    }

    public func request(_ operation: HttpRequest) async -> HttpResult {
        guard let url = URL(string: operation.url) else {
            return .err(.url("could not parse URL: \(operation.url)"))
        }

        var urlRequest = URLRequest(url: url)
        urlRequest.httpMethod = operation.method
        // Header names and values cross the boundary as plain strings, with no
        // validation — `addValue` applies whatever rules it has.
        for header in operation.headers {
            urlRequest.addValue(header.value, forHTTPHeaderField: header.name)
        }
        // An empty body is no body: a GET with `httpBody` set to empty `Data`
        // is not the same request.
        if !operation.body.isEmpty {
            urlRequest.httpBody = Data(operation.body)
        }

        do {
            let (data, response) = try await session.data(for: urlRequest)
            guard let httpResponse = response as? HTTPURLResponse else {
                return .err(.io("not an HTTP response"))
            }
            let headers = (httpResponse.allHeaderFields as? [String: String] ?? [:])
                .map { HttpHeader(name: $0.key, value: $0.value) }
            return .ok(
                HttpResponse(
                    status: UInt16(clamping: httpResponse.statusCode),
                    headers: headers,
                    body: [UInt8](data)
                )
            )
        } catch let error as URLError where error.code == .timedOut {
            return .err(.timeout)
        } catch let error as URLError where error.code == .badURL || error.code == .unsupportedURL {
            return .err(.url(error.localizedDescription))
        } catch {
            return .err(.io(error.localizedDescription))
        }
    }
}
