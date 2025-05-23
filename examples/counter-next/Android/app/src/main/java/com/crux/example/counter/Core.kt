@file:Suppress("NAME_SHADOWING")

package com.crux.example.counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import com.crux.example.counter.shared.Effect
import com.crux.example.counter.shared.Event
import com.crux.example.counter.shared.HttpResult
import com.crux.example.counter.shared.Request
import com.crux.example.counter.shared.Requests
import com.crux.example.counter.shared.ViewModel
import com.crux.example.counter.shared.handleResponse
import com.crux.example.counter.shared.processEvent
import com.crux.example.counter.shared.view
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.engine.cio.endpoint
import kotlinx.coroutines.launch

class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel? by mutableStateOf(null)
        private set

    private val httpClient = HttpClient(CIO)
    private val sseClient =
            HttpClient(CIO) {
                engine {
                    endpoint {
                        keepAliveTime = 5000
                        connectTimeout = 5000
                        connectAttempts = 5
                        requestTimeout = 0
                    }
                }
            }

    init {
        viewModelScope.launch { update(Event.StartWatch()) }
    }

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
            is Effect.ServerSentEvents -> {
                requestSse(sseClient, effect.value) { response ->
                    val effects = handleResponse(request.id.toUInt(), response.bincodeSerialize())

                    val requests = Requests.bincodeDeserialize(effects)
                    for (request in requests) {
                        processEffect(request)
                    }
                }
            }
        }
    }
}
