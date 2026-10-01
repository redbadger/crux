package com.crux.examples.counter.http

import android.util.Log
import com.crux.examples.counter.EffectHandler
import com.crux.examples.counter.EffectSink
import com.crux.examples.counter.HttpHandler
import com.crux.examples.counter.HttpRequest
import com.crux.examples.counter.HttpResult
import com.crux.examples.counter.SseRequest
import com.crux.examples.counter.SseResponse
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlin.coroutines.cancellation.CancellationException

/**
 * The shell's side of the effect protocol: the generated `EffectHandler` has
 * one method per operation the app declares. Nothing here calls `resolve`:
 * the generated `EffectDispatcher` does that, exactly as often as the
 * operation's kind says.
 *
 * There is no `render` override: the generated `Core` intercepts `Render`
 * before the dispatcher sees it and refreshes its `view` flow, so the
 * interface's default no-op stands.
 *
 * @param scope where the Server-Sent Events connection runs, so that it is
 *   closed when the scope is cancelled.
 */
class CounterHandler(
    private val scope: CoroutineScope,
) : EffectHandler {
    /**
     * The app's own [HttpHandler], over Ktor (see `KtorHttpHandler.kt`). It
     * implements the `HttpHandler` shell protocol of `crux_http`, which gives
     * the rules, so `http` is the line that delegates to it.
     */
    private val httpHandler: HttpHandler = KtorHttpHandler()

    private val sseClient = SseClient()

    override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)

    /**
     * `ServerSentEvents` is a stream: every item sent into `sink` is one
     * resolution of the request that opened it.
     *
     * Server-Sent Events are this app's own capability (see
     * `shared/src/sse.rs`), so no crate defines a shell protocol for them and
     * the shell implements the operation here. That is the pattern for any
     * custom capability.
     *
     * The method is not `suspend` (a stream outlives the call that opens
     * it), so the connection runs in a coroutine of its own on `scope`.
     */
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

    companion object {
        private const val TAG = "CounterHandler"
    }
}
