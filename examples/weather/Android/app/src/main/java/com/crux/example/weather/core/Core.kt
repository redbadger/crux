package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.ActiveViewModel
import com.crux.example.weather.CoreFfi
import com.crux.example.weather.Effect
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesViewModel
import com.crux.example.weather.HomeViewModel
import com.crux.example.weather.LocationOperation
import com.crux.example.weather.LocationResult
import com.crux.example.weather.OnboardViewModel
import com.crux.example.weather.Request
import com.crux.example.weather.Requests
import com.crux.example.weather.ViewModel
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.mapNotNull
import kotlinx.coroutines.launch
import javax.inject.Inject
import javax.inject.Singleton

// ANCHOR: core_base
@Singleton
class Core
    @Inject
    constructor(
        private val httpHandler: HttpHandler,
        private val locationHandler: LocationHandler,
        private val keyValueHandler: KeyValueHandler,
        private val secretStore: SecretStore,
        private val timeHandler: TimeHandler,
    ) {
        private val coreFfi = CoreFfi()
        private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

        private val _viewModel: MutableStateFlow<ViewModel> = MutableStateFlow(getViewModel())
        val viewModel: StateFlow<ViewModel> = _viewModel.asStateFlow()

        // ANCHOR: start
        init {
            update(Event.Start)
        }
        // ANCHOR_END: start

        fun homeViewModel(): Flow<HomeViewModel> =
            viewModel.mapNotNull { vm ->
                (vm as? ViewModel.Active)?.let { (it.value as? ActiveViewModel.Home)?.value }
            }

        fun favoritesViewModel(): Flow<FavoritesViewModel> =
            viewModel.mapNotNull { vm ->
                (vm as? ViewModel.Active)?.let { (it.value as? ActiveViewModel.Favorites)?.value }
            }

        fun onboardViewModel(): Flow<OnboardViewModel> =
            viewModel.mapNotNull { (it as? ViewModel.Onboard)?.value }

        fun update(event: Event) {
            Log.d(TAG, "update: $event")
            scope.launch {
                val effects = coreFfi.update(event.bincodeSerialize())
                handleEffects(effects)
            }
        }
        // ANCHOR_END: core_base

        private suspend fun handleEffects(effects: ByteArray) {
            if (effects.isEmpty()) {
                Log.d(TAG, "handleEffects: empty response (no effects)")
                return
            }
            val requests = Requests.bincodeDeserialize(effects)
            for (request in requests) {
                processRequest(request)
            }
        }

        // ANCHOR: process_request
        private suspend fun processRequest(request: Request) {
            Log.d(TAG, "processRequest: $request")

            when (val effect = request.effect) {
                // ANCHOR: http
                is Effect.Http -> {
                    handleHttpEffect(effect, request.id)
                }
                // ANCHOR_END: http

                is Effect.KeyValue -> {
                    handleKeyValueEffect(effect, request.id)
                }

                is Effect.Location -> {
                    handleLocationEffect(effect, request.id)
                }

                is Effect.Secret -> {
                    handleSecretEffect(effect, request.id)
                }

                is Effect.Time -> {
                    // Fire-and-forget: the time handler launches its own coroutines
                    // and resolves asynchronously when timers fire.
                    timeHandler.handle(effect.value, request.id, ::resolveAndHandleEffects)
                }

                is Effect.Render -> {
                    render()
                }
            }
        }
        // ANCHOR_END: process_request

        // ANCHOR: handle_http
        private suspend fun handleHttpEffect(
            effect: Effect.Http,
            requestId: UInt,
        ) {
            val result = httpHandler.request(effect.value)
            resolveAndHandleEffects(requestId, result.bincodeSerialize())
        }
        // ANCHOR_END: handle_http

        private suspend fun handleLocationEffect(
            effect: Effect.Location,
            requestId: UInt,
        ) {
            val result =
                when (effect.value) {
                    LocationOperation.ISLOCATIONENABLED -> {
                        LocationResult.Enabled(locationHandler.isLocationEnabled())
                    }

                    LocationOperation.GETLOCATION -> {
                        LocationResult.Location(locationHandler.getLastLocation())
                    }
                }
            resolveAndHandleEffects(requestId, result.bincodeSerialize())
        }

        private suspend fun handleKeyValueEffect(
            effect: Effect.KeyValue,
            requestId: UInt,
        ) {
            val result = keyValueHandler.handleEffect(effect)
            resolveAndHandleEffects(requestId, result.bincodeSerialize())
        }

        private suspend fun handleSecretEffect(
            effect: Effect.Secret,
            requestId: UInt,
        ) {
            val result = secretStore.handle(effect.value)
            resolveAndHandleEffects(requestId, result.bincodeSerialize())
        }

        // ANCHOR: resolve
        private suspend fun resolveAndHandleEffects(
            requestId: UInt,
            data: ByteArray,
        ) {
            Log.d(TAG, "resolveAndHandleEffects for request id: $requestId")
            val effects = coreFfi.resolve(requestId, data)
            handleEffects(effects)
        }
        // ANCHOR_END: resolve

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
