package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.ActiveViewModel
import com.crux.example.weather.Clear
import com.crux.example.weather.CoreFfi
import com.crux.example.weather.Delete
import com.crux.example.weather.EffectDispatcher
import com.crux.example.weather.EffectHandler
import com.crux.example.weather.Event
import com.crux.example.weather.FavoritesViewModel
import com.crux.example.weather.Fetch
import com.crux.example.weather.Get
import com.crux.example.weather.GetLocation
import com.crux.example.weather.HomeViewModel
import com.crux.example.weather.HttpRequest
import com.crux.example.weather.HttpResult
import com.crux.example.weather.IsLocationEnabled
import com.crux.example.weather.Location
import com.crux.example.weather.NotifyAfter
import com.crux.example.weather.OnboardViewModel
import com.crux.example.weather.RenderOperation
import com.crux.example.weather.Request
import com.crux.example.weather.Requests
import com.crux.example.weather.SecretDeleteResponse
import com.crux.example.weather.SecretFetchResponse
import com.crux.example.weather.SecretStoreResponse
import com.crux.example.weather.Store
import com.crux.example.weather.TimerId
import com.crux.example.weather.ValueResult
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
import com.crux.example.weather.Set as KeyValueSet

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
    ) : EffectHandler {
        private val coreFfi = CoreFfi()
        private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate)

        // The generated dispatcher calls the handler method for each effect and
        // resolves the request with whatever that method returns — never for a
        // notification, exactly once for a request.
        private val dispatcher =
            EffectDispatcher(this) { requestId, data ->
                scope.launch { resolveAndHandleEffects(requestId, data) }
            }

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

        private fun handleEffects(effects: ByteArray) {
            if (effects.isEmpty()) {
                Log.d(TAG, "handleEffects: empty response (no effects)")
                return
            }
            val requests = Requests.bincodeDeserialize(effects).value
            for (request in requests) {
                processRequest(request)
            }
        }

        // ANCHOR: process_request
        /// Each request gets its own coroutine: `dispatch` suspends for as long
        /// as the handler method does, and a debounce timer must not hold up the
        /// requests queued behind it.
        private fun processRequest(request: Request) {
            Log.d(TAG, "processRequest: $request")
            scope.launch { dispatcher.dispatch(request) }
        }
        // ANCHOR_END: process_request

        // ANCHOR: http
        // Every `EffectHandler` method returns the one output its operation is
        // answered with, and nothing here calls `resolve`.
        // ANCHOR: handle_http
        override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)
        // ANCHOR_END: handle_http
        // ANCHOR_END: http

        override fun render(operation: RenderOperation) = render()

        override suspend fun kvGet(operation: Get): ValueResult = keyValueHandler.get(operation)

        override suspend fun kvSet(operation: KeyValueSet): ValueResult = keyValueHandler.set(operation)

        override suspend fun timeNotifyAfter(operation: NotifyAfter): TimerId = timeHandler.notifyAfter(operation)

        override fun timeClear(operation: Clear) = timeHandler.clear(operation)

        override suspend fun isLocationEnabled(operation: IsLocationEnabled): Boolean = locationHandler.isLocationEnabled()

        override suspend fun getLocation(operation: GetLocation): Location? = locationHandler.getLastLocation()

        override suspend fun fetchSecret(operation: Fetch): SecretFetchResponse = secretStore.fetch(operation.value)

        override suspend fun storeSecret(operation: Store): SecretStoreResponse =
            secretStore.store(operation.field0, operation.field1)

        override suspend fun deleteSecret(operation: Delete): SecretDeleteResponse = secretStore.delete(operation.value)

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
