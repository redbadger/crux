package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.CoreFfi
import com.crux.example.weather.Effect
import com.crux.example.weather.Event
import com.crux.example.weather.Request
import com.crux.example.weather.Requests
import com.crux.example.weather.ViewModel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

class Core(
    private val httpClient: HttpClient,
) {
    private val coreFfi = CoreFfi()
    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

    private val _viewModel: MutableStateFlow<ViewModel> = MutableStateFlow(getViewModel())
    val viewModel: StateFlow<ViewModel> = _viewModel.asStateFlow()

    fun update(event: Event) {
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
        Log.d(TAG, "processRequest: ${request.id}, ${request.effect}")
        when (val effect = request.effect) {
            is Effect.Http -> {
                handleHttpEffect(effect, request.id)
            }

            is Effect.KeyValue -> {
                // TODO
            }

            is Effect.Location -> {
                // TODO
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

    private suspend fun resolveAndHandleEffects(requestId: UInt, data: ByteArray) {
        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private fun render() {
        _viewModel.value = getViewModel()
    }

    private fun getViewModel(): ViewModel {
        return ViewModel.bincodeDeserialize(coreFfi.view())
    }

    companion object Companion {
        private const val TAG = "CoreStore"
    }
}
