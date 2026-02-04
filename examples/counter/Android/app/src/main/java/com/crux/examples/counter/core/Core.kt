package com.crux.examples.counter.core

import android.util.Log
import com.crux.examples.counter.CoreFfi
import com.crux.examples.counter.Effect
import com.crux.examples.counter.Event
import com.crux.examples.counter.Request
import com.crux.examples.counter.Requests
import com.crux.examples.counter.ViewModel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class Core(
    private val httpClient: HttpClient,
    private val sseClient: SseClient,
) {
    private val coreFfi = CoreFfi()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    private val _viewModel: MutableStateFlow<ViewModel> = MutableStateFlow(getViewModel())
    val viewModel: StateFlow<ViewModel> = _viewModel.asStateFlow()

    fun update(event: Event) {
        Log.d(TAG, "update: $event")

        scope.launch {
            val effects = coreFfi.update(event.bincodeSerialize())
            handleEffects(effects)
        }
    }

    private suspend fun handleEffects(effects: ByteArray) {
        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processRequest(request)
        }
    }

    private suspend fun processRequest(request: Request) {
        Log.d(TAG, "processRequest: $request")

        when (val effect = request.effect) {
            is Effect.Http -> {
                handleHttpEffect(effect, request.id)
            }

            is Effect.ServerSentEvents -> {
                handleSseEffect(effect, request.id)
            }

            is Effect.Render -> {
                render()
            }
        }
    }

    private suspend fun handleHttpEffect(effect: Effect.Http, requestId: UInt) {
        val result = httpClient.request(effect.value)
        resolveAndHandleEffects(requestId, result.bincodeSerialize())
    }

    private suspend fun handleSseEffect(effect: Effect.ServerSentEvents, requestId: UInt) {
        sseClient.request(effect.value) { response ->
            Log.d("crux", response.toString())
            resolveAndHandleEffects(requestId, response.bincodeSerialize())
        }
    }

    private suspend fun resolveAndHandleEffects(requestId: UInt, data: ByteArray) {
        Log.d(TAG, "resolveAndHandleEffects for request id: $requestId")

        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private fun render() {
        _viewModel.value = getViewModel().also {
            Log.d(TAG, "render: $it")
        }
    }

    private fun getViewModel(): ViewModel {
        return ViewModel.bincodeDeserialize(coreFfi.view())
    }

    companion object {
        private const val TAG = "Core"
    }
}
