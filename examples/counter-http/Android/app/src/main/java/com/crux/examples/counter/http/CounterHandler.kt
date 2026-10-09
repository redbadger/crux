package com.crux.examples.counter.http

import android.util.Log
import com.crux.examples.counter.ClearTimer
import com.crux.examples.counter.CoroutineTimeHandler
import com.crux.examples.counter.EffectHandler
import com.crux.examples.counter.EffectSink
import com.crux.examples.counter.HttpHandler
import com.crux.examples.counter.HttpRequest
import com.crux.examples.counter.HttpResult
import com.crux.examples.counter.NotifyAfter
import com.crux.examples.counter.SseRequest
import com.crux.examples.counter.SseResponse
import com.crux.examples.counter.TimeHandler
import com.crux.examples.counter.TimerId
import com.crux.examples.counter.UrlConnectionHttpHandler
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import kotlin.coroutines.cancellation.CancellationException

/// The shell's side of the effect protocol: the generated `EffectHandler` has
/// one method per operation the app declares. Nothing here calls `resolve`:
/// the generated `EffectDispatcher` does that, exactly as often as the
/// operation's kind says.
///
/// There is no `render` override: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and refreshes its `view` flow, so the
/// interface's default no-op stands.
///
/// @param scope where the Server-Sent Events connection and the timers run, so
///   that they are stopped when the scope is cancelled.
class CounterHandler(
    private val scope: CoroutineScope,
) : EffectHandler {
    /// The handler `crux_http` ships, generated into this module: the rules
    /// for mapping a response belong to that crate, so the only thing this
    /// shell writes for HTTP is the line that delegates.
    private val httpHandler: HttpHandler = UrlConnectionHttpHandler()

    /// The handler `crux_time` ships. It owns the timer table, so there is
    /// one of it, made here.
    private val timeHandler: TimeHandler = CoroutineTimeHandler(scope)

    private val sseClient = SseClient()

    override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)

    override suspend fun timeNotifyAfter(operation: NotifyAfter): TimerId = timeHandler.notifyAfter(operation)

    override suspend fun timeClear(operation: ClearTimer): TimerId = timeHandler.clear(operation)

    /// `ServerSentEvents` is a stream: every item sent into `sink` is one
    /// resolution of the request that opened it.
    ///
    /// Server-Sent Events are this app's own capability (see
    /// `shared/src/sse.rs`), so no crate ships a handler for them and the shell
    /// implements the operation here. That is the pattern for any custom
    /// capability.
    ///
    /// The method is not `suspend` (a stream outlives the call that opens
    /// it), so the connection runs in a coroutine of its own on `scope`. If
    /// the connection fails, the stream still ends with `Done`, so the core
    /// knows to reopen it.
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
                sink.send(SseResponse.Done)
            }
        }
    }

    companion object {
        private const val TAG = "CounterHandler"
    }
}
