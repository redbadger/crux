@file:Suppress("NAME_SHADOWING")

package com.redbadger.catfacts

import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.crux.example.cat_facts.Effect
import com.crux.example.cat_facts.Event
import com.crux.example.cat_facts.HttpResult
import com.crux.example.cat_facts.Instant
import com.crux.example.cat_facts.PlatformResponse
import com.crux.example.cat_facts.Request
import com.crux.example.cat_facts.Requests
import com.crux.example.cat_facts.TimeResponse
import com.crux.example.cat_facts.ViewModel
import com.redbadger.catfacts.shared.handleResponse
import com.redbadger.catfacts.shared.processEvent
import com.redbadger.catfacts.shared.view
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

                val effects = handleResponse(request.id.toUInt(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }
            is Effect.Platform -> {
                val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)

                val effects = handleResponse(request.id.toUInt(), response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }
            is Effect.KeyValue -> {}
        }
    }
}
