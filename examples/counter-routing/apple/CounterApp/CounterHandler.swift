import App
import Foundation

/// `CounterHandler` is the shell's side of the effect protocol: it implements
/// the generated `EffectHandler`, which has one method per operation the app
/// declares. For `http` and `serverSentEvents` nothing here calls `resolve`:
/// the generated `EffectDispatcher` does that, exactly as often as the
/// operation's kind says.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it, so the protocol's default no-op stands.
struct CounterHandler: EffectHandler {
    /// The shipped `crux_http` handler, over the shared `URLSession`.
    let httpHandler = URLSessionHttpHandler.shared

    /// `Http` is a request: the shell performs it and answers with exactly one
    /// `HttpResult`. HTTP is `crux_http`'s business, so the rules for
    /// producing one belong to the handler it ships, and this line delegates.
    func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }

    /// `ServerSentEvents` is a stream: every item sent into `sink` is one
    /// resolution of the request that opened it.
    ///
    /// Server-Sent Events are this app's own capability (see
    /// `shared/src/capabilities/sse.rs`), so no crate ships a handler for them
    /// and the shell implements the operation here.
    ///
    /// Each complete event (the bytes up to a blank line) goes back as a
    /// `chunk`, for the core to decode. When the server closes the connection
    /// the stream ends with `done`, and nothing is sent after it.
    func serverSentEvents(_ operation: SseRequest, into sink: EffectSink<SseResponse>) {
        guard let url = URL(string: operation.url) else { return }

        Task {
            do {
                let (bytes, _) = try await URLSession.shared.bytes(from: url)
                var buffer = Data()

                for try await byte in bytes {
                    buffer.append(byte)
                    if buffer.suffix(2) == Data([0x0A, 0x0A]) {
                        sink.send(.chunk([UInt8](buffer)))
                        buffer = Data()
                    }
                }

                sink.send(.done)
            } catch {
                print("SSE error: \(error)")
            }
        }
    }

    /// `Random` is answered in Rust: the `EffectRouter` in `shared/src/ffi.rs`
    /// routes it to the `RngHandler` in `shared/src/rng_handler.rs`, so it
    /// never reaches this shell. The protocol still asks for the method,
    /// because type generation describes the app's `Effect`, which has the
    /// variant, not what the router leaves of it.
    func random(_ operation: RandomNumberRequest) async -> RandomNumber {
        fatalError("Random is handled by the router and should not reach the shell")
    }
}
