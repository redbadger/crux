package com.crux.examples.counter.http

import com.crux.examples.counter.ClearTimer
import com.crux.examples.counter.NotifyAfter
import com.crux.examples.counter.TimerId
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlin.coroutines.coroutineContext

/**
 * Runs the timers of `crux_time` with `delay` in a coroutine.
 *
 * The handler keeps a table of the timers that run, so that a [ClearTimer]
 * can cancel the timer that it names.
 */
class CoroutineTimeHandler {
    private val activeTimers = mutableMapOf<ULong, Job>()

    /**
     * [NotifyAfter] is answered exactly once, with the id of the timer that
     * fired.
     *
     * If a [ClearTimer] for the timer arrives first, it cancels the coroutine
     * that runs this method, so this method does not answer. The core stops
     * listening for this request when it clears the timer.
     */
    suspend fun notifyAfter(operation: NotifyAfter): TimerId {
        val timerId = operation.id.value
        val delayMs = (operation.duration.nanos / 1_000_000u).toLong()

        activeTimers[timerId] = checkNotNull(coroutineContext[Job]) { "no job to cancel" }
        try {
            delay(delayMs)
        } finally {
            activeTimers.remove(timerId)
        }

        return operation.id
    }

    /** [ClearTimer] cancels the timer and answers with the id that it names. */
    fun clear(operation: ClearTimer): TimerId {
        activeTimers.remove(operation.id.value)?.cancel()

        return operation.id
    }
}
