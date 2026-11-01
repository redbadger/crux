import Foundation
import FoundationNetworking

/// An `HttpHandler` over `URLSession`.
///
/// The app's handler holds an instance and delegates the operation to it:
///
/// ```swift
/// struct MyHandler: EffectHandler {
///     let http = URLSessionHttpHandler()
///
///     func http(_ operation: HttpRequest) async -> HttpResult {
///         await http.request(operation)
///     }
/// }
/// ```
///
/// ## What is a failure
///
/// The handler follows the rules of `HttpHandler`. A URL that `URL(string:)`
/// cannot parse is `HttpError.url`, and so are `URLError.badURL` and
/// `URLError.unsupportedURL`. `URLError.timedOut` is `HttpError.timeout`.
///
/// ## Redirects
///
/// The session follows redirects by its own rules, and the core gets the last
/// response. A session delegate can change those rules.
///
/// ## Cookies
///
/// The session keeps cookies, so a session survives between requests. With the
/// default `URLSession.shared`, they are in the process-wide
/// `HTTPCookieStorage.shared`. A session of your own uses the storage in its
/// configuration.
public final class URLSessionHttpHandler: HttpHandler {
    private let session: URLSession

    /// Performs every request on `session`.
    ///
    /// With no argument that is `URLSession.shared`, so every handler made this
    /// way shares one session, and its cookies are kept in the process-wide
    /// `HTTPCookieStorage.shared`. A pinned session, a session with a proxy
    /// configuration, or one with a delegate of your own is the shell's
    /// business, and this is where it goes.
    public init(session: URLSession = .shared) {
        self.session = session
    }

    public func request(_ operation: HttpRequest) async -> HttpResult {
        guard let url = URL(string: operation.url) else {
            return .err(.url("could not parse URL: \(operation.url)"))
        }

        let method = operation.method.uppercased()
        if !operation.body.isEmpty, method == "GET" || method == "HEAD" {
            return .err(.io("\(method) request cannot carry a body"))
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
