package com.crux.example.counter

import android.util.Log
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.viewModelScope
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import io.ktor.client.engine.cio.endpoint
import kotlinx.coroutines.launch
import uniffi.crux_http.HttpError
import uniffi.crux_http.HttpResult

class Core : androidx.lifecycle.ViewModel(), NativeShell {
    private val core = CoreFfi(this)
    var view: ViewModel by mutableStateOf(core.view())
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
        update(EventFfi.START_WATCH)
    }

    fun update(event: EventFfi) {
        core.update(event)
    }

    override fun handleEffect(request: NativeRequest) {
        when (val effect = request.effect) {
            is EffectFfi.Render -> {
                view = core.view()
            }
            is EffectFfi.Http -> {
                viewModelScope.launch {
                    val result =
                        try {
                            HttpResult.Ok(requestHttp(httpClient, effect.v1))
                        } catch (e: Exception) {
                            Log.e("Core", "HTTP request failed", e)
                            HttpResult.Err(HttpError.Io(e.message ?: "unknown error"))
                        }
                    core.resolve(request.id, EffectOutput.Http(result))
                }
            }
            is EffectFfi.ServerSentEvents -> {
                viewModelScope.launch {
                    try {
                        requestSse(sseClient, effect.v1) { response ->
                            core.resolve(request.id, EffectOutput.ServerSentEvents(response))
                        }
                    } catch (e: Exception) {
                        Log.e("Core", "SSE request failed", e)
                    }
                }
            }
        }
    }
}
