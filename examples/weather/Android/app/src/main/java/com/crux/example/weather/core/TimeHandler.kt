package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.Clear
import com.crux.example.weather.NotifyAfter
import com.crux.example.weather.TimerId
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import javax.inject.Inject
import javax.inject.Singleton
import kotlin.coroutines.coroutineContext

@Singleton
class TimeHandler
    @Inject
    constructor() {
        private val activeTimers = mutableMapOf<ULong, Job>()

        /// `NotifyAfter` is answered exactly once, with the id of the timer that
        /// fired.
        ///
        /// If `Clear` arrives first the coroutine that is dispatching this
        /// request is cancelled, so it never answers: `Clear` is a notification,
        /// which means the core has already stopped waiting for the timer.
        suspend fun notifyAfter(operation: NotifyAfter): TimerId {
            val timerId = operation.id.value
            val delayMs = (operation.duration.nanos / 1_000_000u).toLong()
            Log.d(TAG, "notifyAfter: $delayMs ms (id=$timerId)")

            activeTimers[timerId] = checkNotNull(coroutineContext[Job]) { "no job to cancel" }
            try {
                delay(delayMs)
            } finally {
                activeTimers.remove(timerId)
            }

            Log.d(TAG, "timer elapsed (id=$timerId)")
            return operation.id
        }

        /// `Clear` is a notification: drop the timer and answer nothing.
        fun clear(operation: Clear) {
            val timerId = operation.id.value
            Log.d(TAG, "clear (id=$timerId)")
            activeTimers.remove(timerId)?.cancel()
        }

        companion object {
            private const val TAG = "TimeHandler"
        }
    }
