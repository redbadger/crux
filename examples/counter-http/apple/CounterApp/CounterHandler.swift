import App
import Foundation

/// `CounterHandler` is the shell's side of the effect protocol: it implements
/// the generated `EffectHandler`, which has one method per operation the app
/// declares. Nothing here calls `resolve`: the generated `EffectDispatcher`
/// does that, exactly as often as the operation's kind says.
///
/// There is no `render` method: the generated `Core` intercepts `Render`
/// before the dispatcher sees it, so the protocol's default no-op stands.
struct CounterHandler: EffectHandler {
    /// The shell's own HTTP client, over the shared `URLSession` (see
    /// `http.swift`).
    let httpHandler = SessionHttpHandler()

    /// The shell's own timers, as sleeping tasks (see `time.swift`). The
    /// handler owns the timer table, so there is one of it, made here.
    let timeHandler = TaskTimeHandler()

    /// `Http` is a request: the shell performs it and answers with exactly one
    /// `HttpResult`. `SessionHttpHandler` sends the request, so this line
    /// delegates.
    func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }

    /// `TimeNotifyAfter` is answered exactly once, with the id of the timer
    /// that fired. `TaskTimeHandler` runs the timers, so these two delegate.
    func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await timeHandler.notifyAfter(operation)
    }

    /// `TimeClear` cancels the timer and answers with the id it named.
    func timeClear(_ operation: ClearTimer) async -> TimerId {
        await timeHandler.clear(operation)
    }

    /// `ServerSentEvents` is a stream: every item sent into `sink` is one
    /// resolution of the request that opened it.
    ///
    /// Server-Sent Events are this app's own capability (see
    /// `shared/src/sse.rs`), so the shell implements the operation here. That is the pattern for any
    /// custom capability.
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
}
