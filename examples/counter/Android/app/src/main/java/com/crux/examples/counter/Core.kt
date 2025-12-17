package com.crux.examples.counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.engine.cio.endpoint
import kotlinx.coroutines.launch

open class Core : androidx.lifecycle.ViewModel() {
    private var core: CoreFfi = CoreFfi()

    private val httpClient = HttpClient(CIO)
    private val sseClient = HttpClient(CIO) {
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
        viewModelScope.launch {
            update(Event.STARTWATCH)
        }
    }

    var view: ViewModel by mutableStateOf(
        ViewModel.bincodeDeserialize(core.view())
    )
        private set

    suspend fun update(event: Event) {
        val effects = core.update(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private suspend fun processEffect(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(core.view())
            }
            is Effect.Http -> {
                val response = requestHttp(httpClient, effect.value)

                val effects =
                    core.resolve(
                        request.id,
                        HttpResult.Ok(response).bincodeSerialize()
                    )

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.ServerSentEvents -> {
                requestSse(sseClient, effect.value) { response ->
                    val effects =
                        core.resolve(request.id, response.bincodeSerialize())

                    val requests = Requests.bincodeDeserialize(effects)
                    for (request in requests) {
                        processEffect(request)
                    }
                }
            }
        }
    }
}


