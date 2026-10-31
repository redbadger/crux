import Foundation

/// The shell side of `crux_http`, as one method per operation.
///
/// The method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
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
/// Conform your own type to it to replace the implementation wholesale, in the
/// app or in a test.
public protocol HttpHandler: Sendable {
    /// Perform `operation` and answer with its result, exactly once.
    func request(_ operation: HttpRequest) async -> HttpResult
}
