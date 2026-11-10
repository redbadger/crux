package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.ClearTimer
import com.crux.example.weather.EffectSink
import com.crux.example.weather.Instant
import com.crux.example.weather.NotifyAfter
import com.crux.example.weather.NotifyAt
import com.crux.example.weather.NotifyEvery
import com.crux.example.weather.Now
import com.crux.example.weather.TimeHandler
import com.crux.example.weather.TimerId
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.ConcurrentHashMap

/**
 * The app's [TimeHandler], over coroutine timeouts.
 *
 * The handler answers each one-shot timer exactly once, with the id of the
 * timer. If a `ClearTimer` arrives first, the handler wakes the pending call,
 * which then answers immediately. The core ignores that answer, because it no
 * longer listens for it.
 *
 * A repeating timer is a coroutine on [scope] that waits for one period, then
 * sends the id of the timer into the sink, until a `ClearTimer` cancels the
 * coroutine.
 *
 * @param scope the scope that the repeating timers run in.
 */
class DelayTimeHandler(
    private val scope: CoroutineScope = CoroutineScope(SupervisorJob() + Dispatchers.Default),
) : TimeHandler {
    /** The pending timers, so that `clear` can wake the timer that it names. */
    private val timers = ConcurrentHashMap<ULong, CompletableDeferred<Unit>>()

    /** The repeating timers, so that `clear` can cancel the timer that it names. */
    private val repeating = ConcurrentHashMap<ULong, Job>()

    override suspend fun now(operation: Now): Instant {
        val now = java.time.Instant.now()
        return Instant(now.epochSecond.toULong(), now.nano.toUInt())
    }

    override suspend fun notifyAt(operation: NotifyAt): TimerId {
        val at = operation.instant.seconds.toLong() * 1_000 + operation.instant.nanos.toLong() / 1_000_000
        return wait(operation.id, (at - System.currentTimeMillis()).coerceAtLeast(0))
    }

    override suspend fun notifyAfter(operation: NotifyAfter): TimerId =
        wait(operation.id, (operation.duration.nanos / 1_000_000u).toLong())

    override fun notifyEvery(
        operation: NotifyEvery,
        sink: EffectSink<TimerId>,
    ) {
        val id = operation.id
        // A period of 0 would make the loop spin.
        val periodMs = (operation.period.nanos / 1_000_000u).toLong().coerceAtLeast(1)
        Log.d(TAG, "start: every $periodMs ms (id=${id.value})")
        // Lazy, so that the timer is in the table before it can run.
        val job =
            scope.launch(start = CoroutineStart.LAZY) {
                while (true) {
                    // `clear` cancels the coroutine, which also ends the delay.
                    delay(periodMs)
                    sink.send(id)
                }
            }
        repeating[id.value] = job
        job.invokeOnCompletion { repeating.remove(id.value, job) }
        job.start()
    }

    override suspend fun clear(operation: ClearTimer): TimerId {
        Log.d(TAG, "clear (id=${operation.id.value})")
        timers.remove(operation.id.value)?.complete(Unit)
        repeating.remove(operation.id.value)?.cancel()
        return operation.id
    }

    private suspend fun wait(
        id: TimerId,
        delayMs: Long,
    ): TimerId {
        Log.d(TAG, "start: $delayMs ms (id=${id.value})")
        val cleared = CompletableDeferred<Unit>()
        timers[id.value] = cleared
        try {
            withTimeoutOrNull(delayMs) { cleared.await() }
        } finally {
            timers.remove(id.value, cleared)
        }
        Log.d(TAG, "answer (id=${id.value})")
        return id
    }

    companion object {
        private const val TAG = "DelayTimeHandler"
    }
}
