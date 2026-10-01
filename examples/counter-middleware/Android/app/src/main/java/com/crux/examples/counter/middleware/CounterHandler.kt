package com.crux.examples.counter.middleware

import android.util.Log
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlin.coroutines.cancellation.CancellationException

/// The shell's side of the effect protocol: the generated `EffectHandler` has
/// one method per operation the app declares. For `http` and
/// `serverSentEvents` nothing here calls `resolve`: the generated
/// `EffectDispatcher` does that, exactly as often as the operation's kind
/// says.
///
/// There is no `render` override: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and refreshes its `view` flow, so the
/// interface's default no-op stands.
///
/// @param scope where the Server-Sent Events connection runs, so that it is
///   closed when the scope is cancelled.
class CounterHandler(
    private val scope: CoroutineScope,
) : EffectHandler {
    /// The handler `crux_http` ships, generated into this module: the rules
    /// for mapping a response belong to that crate, so the only thing this
    /// shell writes for HTTP is the line that delegates.
    private val httpHandler: HttpHandler = UrlConnectionHttpHandler()

    private val sseClient = SseClient()

    override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)

    /// `ServerSentEvents` is a stream: every item sent into `sink` is one
    /// resolution of the request that opened it.
    ///
    /// Server-Sent Events are this app's own capability (see
    /// `shared/src/capabilities/sse.rs`), so no crate ships a handler for
    /// them and the shell implements the operation here.
    ///
    /// The method is not `suspend` (a stream outlives the call that opens
    /// it), so the connection runs in a coroutine of its own on `scope`.
    override fun serverSentEvents(
        operation: SseRequest,
        sink: EffectSink<SseResponse>,
    ) {
        scope.launch {
            try {
                sseClient.request(operation) { sink.send(it) }
            } catch (ce: CancellationException) {
                throw ce
            } catch (error: Exception) {
                Log.e(TAG, "SSE error", error)
            }
        }
    }

    /// `Random` is answered in Rust, by the `RngMiddleware` in
    /// `shared/src/middleware.rs`, so it never reaches this shell. The
    /// interface still asks for the method, because type generation describes
    /// the app's `Effect`, which has the variant, not what the middleware
    /// leaves of it.
    override suspend fun random(operation: RandomNumberRequest): RandomNumber =
        error("Random is handled by the middleware and should not reach the shell")

    companion object {
        private const val TAG = "CounterHandler"
    }
}
