package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.Instant
import com.crux.example.weather.TimeRequest
import com.crux.example.weather.TimeResponse
import com.crux.example.weather.TimerId
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class TimeHandler(private val scope: CoroutineScope) {

    private val activeTimers = mutableMapOf<ULong, Job>()

    fun handle(
        request: TimeRequest,
        requestId: UInt,
        resolve: suspend (UInt, ByteArray) -> Unit,
    ) {
        Log.d(TAG, "handle: $request")
        when (request) {
            is TimeRequest.Now -> {
                val now = System.currentTimeMillis()
                val response = TimeResponse.Now(
                    Instant(
                        seconds = (now / 1000).toULong(),
                        nanos = ((now % 1000) * 1_000_000).toUInt(),
                    )
                )
                scope.launch { resolve(requestId, response.bincodeSerialize()) }
            }

            is TimeRequest.NotifyAt -> {
                val targetMs = request.instant.seconds.toLong() * 1000 +
                    request.instant.nanos.toLong() / 1_000_000
                val delayMs = (targetMs - System.currentTimeMillis()).coerceAtLeast(0)
                val timerId = request.id.value
                activeTimers[timerId] = scope.launch {
                    delay(delayMs)
                    activeTimers.remove(timerId)
                    val response = TimeResponse.InstantArrived(request.id)
                    resolve(requestId, response.bincodeSerialize())
                }
            }

            is TimeRequest.NotifyAfter -> {
                val delayMs = request.duration.nanos / 1_000_000u
                val timerId = request.id.value
                activeTimers[timerId] = scope.launch {
                    delay(delayMs.toLong())
                    activeTimers.remove(timerId)
                    val response = TimeResponse.DurationElapsed(request.id)
                    resolve(requestId, response.bincodeSerialize())
                }
            }

            is TimeRequest.Clear -> {
                val timerId = request.id.value
                activeTimers.remove(timerId)?.cancel()
                val response = TimeResponse.Cleared(request.id)
                scope.launch { resolve(requestId, response.bincodeSerialize()) }
            }
        }
    }

    companion object {
        private const val TAG = "TimeHandler"
    }
}
