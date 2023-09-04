@file:Suppress("NAME_SHADOWING")

package com.example.android

import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.example.counter.requestHttp
import com.redbadger.catfacts.shared.handleResponse
import com.redbadger.catfacts.shared.processEvent
import com.redbadger.catfacts.shared.view
import com.redbadger.catfacts.shared_types.Effect
import com.redbadger.catfacts.shared_types.Event
import com.redbadger.catfacts.shared_types.PlatformResponse
import com.redbadger.catfacts.shared_types.Request
import com.redbadger.catfacts.shared_types.Requests
import com.redbadger.catfacts.shared_types.TimeResponse
import com.redbadger.catfacts.shared_types.ViewModel
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import java.time.ZoneOffset
import java.time.ZonedDateTime
import java.time.format.DateTimeFormatter

open class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel by mutableStateOf(ViewModel.bincodeDeserialize(view()))
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
                    handleResponse(request.uuid.toByteArray(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.Time -> {
                val response =
                    TimeResponse(
                        ZonedDateTime.now(ZoneOffset.UTC).format(DateTimeFormatter.ISO_INSTANT)
                    )

                val effects =
                    handleResponse(request.uuid.toByteArray(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.Platform -> {
                val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)

                val effects =
                    handleResponse(request.uuid.toByteArray(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.KeyValue -> {}
        }
    }
}

