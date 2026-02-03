package com.crux.example.cat_facts.core

import android.os.Build
import android.util.Log
import com.crux.example.cat_facts.CoreFfi
import com.crux.example.cat_facts.Effect
import com.crux.example.cat_facts.Event
import com.crux.example.cat_facts.Instant
import com.crux.example.cat_facts.PlatformResponse
import com.crux.example.cat_facts.Request
import com.crux.example.cat_facts.Requests
import com.crux.example.cat_facts.TimeResponse
import com.crux.example.cat_facts.ViewModel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.time.ZoneOffset
import java.time.ZonedDateTime

class Core(
    private val httpClient: HttpClient,
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

            is Effect.Render -> {
                render()
            }

            is Effect.Time -> {
                handleTimeEffect(request.id)
            }

            is Effect.Platform -> {
                handlePlatformEffect(request.id)
            }

            is Effect.KeyValue -> {}
        }
    }

    private suspend fun handleHttpEffect(
        effect: Effect.Http,
        requestId: UInt,
    ) {
        val result = httpClient.request(effect.value)
        resolveAndHandleEffects(requestId, result.bincodeSerialize())
    }

    private suspend fun handleTimeEffect(requestId: UInt) {
        val now = ZonedDateTime.now(ZoneOffset.UTC)
        val response =
            TimeResponse.Now(
                Instant(
                    now.toEpochSecond().toULong(),
                    now.nano.toUInt(),
                ),
            )
        resolveAndHandleEffects(requestId, response.bincodeSerialize())
    }

    private suspend fun handlePlatformEffect(requestId: UInt) {
        val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)
        resolveAndHandleEffects(requestId, response.bincodeSerialize())
    }

    private suspend fun resolveAndHandleEffects(
        requestId: UInt,
        data: ByteArray,
    ) {
        Log.d(TAG, "resolveAndHandleEffects for request id: $requestId")

        val effects = coreFfi.resolve(requestId, data)
        handleEffects(effects)
    }

    private fun render() {
        _viewModel.value =
            getViewModel().also {
                Log.d(TAG, "render: $it")
            }
    }

    private fun getViewModel(): ViewModel = ViewModel.bincodeDeserialize(coreFfi.view())

    companion object {
        private const val TAG = "Core"
    }
}
