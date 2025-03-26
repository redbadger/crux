@file:Suppress("NAME_SHADOWING")

package com.redbadger.catfacts

import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.redbadger.catfacts.shared.handleResponse
import com.redbadger.catfacts.shared.processEvent
import com.redbadger.catfacts.shared.view
import com.crux.shared.types.Effect
import com.crux.shared.types.Event
import com.crux.shared.types.HttpResult
import com.crux.shared.types.Instant
import com.crux.shared.types.PlatformResponse
import com.crux.shared.types.Request
import com.crux.shared.types.Requests
import com.crux.shared.types.TimeResponse
import com.crux.shared.types.ViewModel
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import java.time.ZoneOffset
import java.time.ZonedDateTime

open class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel? by mutableStateOf(null)
        private set

    private val httpClient = HttpClient(CIO)

    suspend fun update(event: Event) {
        val effects = processEvent(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private suspend fun processEffect(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(view())
            }
            is Effect.Http -> {
                val response = requestHttp(httpClient, effect.value)

                val effects =
                        handleResponse(
                                request.id.toUInt(),
                                HttpResult.Ok(response).bincodeSerialize()
                        )

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }
            is Effect.Time -> {
                val now = ZonedDateTime.now(ZoneOffset.UTC)
                val response = TimeResponse.now(Instant(now.toEpochSecond(), now.nano))

                val effects =
                        handleResponse(request.id.toUInt(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }
            is Effect.Platform -> {
                val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)

                val effects =
                        handleResponse(request.id.toUInt(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }
            is Effect.KeyValue -> {}
        }
    }
}
