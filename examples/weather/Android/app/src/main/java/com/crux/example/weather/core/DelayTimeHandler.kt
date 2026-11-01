package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.ClearTimer
import com.crux.example.weather.Instant
import com.crux.example.weather.NotifyAfter
import com.crux.example.weather.NotifyAt
import com.crux.example.weather.Now
import com.crux.example.weather.TimeHandler
import com.crux.example.weather.TimerId
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.ConcurrentHashMap

/**
 * The app's [TimeHandler], over coroutine timeouts.
 *
 * The handler answers each timer exactly once, with the id of the timer. If a
 * `ClearTimer` arrives first, the handler wakes the pending call, which then
 * answers immediately. The core ignores that answer, because it no longer
 * listens for it.
 */
class DelayTimeHandler : TimeHandler {
    /** The pending timers, so that `clear` can wake the timer that it names. */
    private val timers = ConcurrentHashMap<ULong, CompletableDeferred<Unit>>()

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

    override suspend fun clear(operation: ClearTimer): TimerId {
        Log.d(TAG, "clear (id=${operation.id.value})")
        timers.remove(operation.id.value)?.complete(Unit)
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
