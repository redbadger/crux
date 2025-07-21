@file:Suppress("NAME_SHADOWING")

package com.crux.example.counter

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import com.crux.example.counter.shared.CoreFfi
import com.crux.example.counter.shared.CruxShell
import com.crux.example.counter.shared.Effect
import com.crux.example.counter.shared.Event
import com.crux.example.counter.shared.HttpResult
import com.crux.example.counter.shared.Request
import com.crux.example.counter.shared.Requests
import com.crux.example.counter.shared.view_model.ViewModel
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.engine.cio.endpoint
import kotlinx.coroutines.launch

class Core : androidx.lifecycle.ViewModel(), CruxShell  {
    private var core: CoreFfi = CoreFfi(this)
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
                    val effects = core.resolve(request.id.toUInt(), response.bincodeSerialize())

                    val requests = Requests.bincodeDeserialize(effects)
                    for (request in requests) {
                        processEffect(request)
                    }
                }
            }
        }
    }

    override fun processEffects(bytes: ByteArray) {
        val requests = Requests.bincodeDeserialize(bytes)
        for (request in requests) {
            viewModelScope.launch {
                processEffect(request)
            }
        }
    }
}
