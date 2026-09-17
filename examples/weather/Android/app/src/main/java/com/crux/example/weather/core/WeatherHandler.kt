package com.crux.example.weather.core

import com.crux.example.weather.Clear
import com.crux.example.weather.DeleteSecret
import com.crux.example.weather.EffectHandler
import com.crux.example.weather.FetchSecret
import com.crux.example.weather.Get
import com.crux.example.weather.GetLocation
import com.crux.example.weather.HttpHandler
import com.crux.example.weather.HttpRequest
import com.crux.example.weather.HttpResult
import com.crux.example.weather.IsLocationEnabled
import com.crux.example.weather.KeyValueHandler
import com.crux.example.weather.Location
import com.crux.example.weather.NotifyAfter
import com.crux.example.weather.SecretDeleteResponse
import com.crux.example.weather.SecretFetchResponse
import com.crux.example.weather.SecretStoreResponse
import com.crux.example.weather.StoreSecret
import com.crux.example.weather.TimeHandler
import com.crux.example.weather.TimerId
import com.crux.example.weather.ValueResult
import javax.inject.Inject
import javax.inject.Singleton
import com.crux.example.weather.Set as KeyValueSet

/// The shell's side of the effect protocol: one method per operation the app
/// declares, each returning the single output that operation is answered with.
/// Nothing here calls `resolve` — the generated `EffectDispatcher` does that,
/// exactly as often as the operation's kind says.
///
/// There is no `render` override: the generated `Core` intercepts `Render`
/// before the dispatcher sees it and refreshes its `view` flow, so the
/// interface's default no-op stands.
///
/// `HttpHandler`, `KeyValueHandler` and `TimeHandler` are the handlers
/// `crux_http`, `crux_kv` and `crux_time` ship, generated into this module and
/// injected by `AppModule`: the rules for mapping a response, answering a
/// store operation or answering a timer belong to those crates, so the only
/// thing this shell writes for them is the line that delegates.
@Singleton
class WeatherHandler
    @Inject
    constructor(
        private val httpHandler: HttpHandler,
        private val locationHandler: LocationHandler,
        private val keyValueHandler: KeyValueHandler,
        private val secretStore: SecretStore,
        private val timeHandler: TimeHandler,
    ) : EffectHandler {
        // ANCHOR: handle_http
        override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)
        // ANCHOR_END: handle_http

        override suspend fun kvGet(operation: Get): ValueResult = keyValueHandler.get(operation)

        override suspend fun kvSet(operation: KeyValueSet): ValueResult = keyValueHandler.set(operation)

        override suspend fun timeNotifyAfter(operation: NotifyAfter): TimerId = timeHandler.notifyAfter(operation)

        override suspend fun timeClear(operation: Clear): TimerId = timeHandler.clear(operation)

        override suspend fun isLocationEnabled(operation: IsLocationEnabled): Boolean = locationHandler.isLocationEnabled()

        override suspend fun getLocation(operation: GetLocation): Location? = locationHandler.getLastLocation()

        override suspend fun fetchSecret(operation: FetchSecret): SecretFetchResponse = secretStore.fetch(operation.value)

        override suspend fun storeSecret(operation: StoreSecret): SecretStoreResponse =
            secretStore.store(operation.field0, operation.field1)

        override suspend fun deleteSecret(operation: DeleteSecret): SecretDeleteResponse = secretStore.delete(operation.value)
    }
