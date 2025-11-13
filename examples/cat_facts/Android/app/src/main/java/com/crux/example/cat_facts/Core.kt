package com.crux.example.cat_facts

import android.os.Build
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import io.ktor.client.HttpClient
import io.ktor.client.engine.cio.CIO
import java.time.ZoneOffset
import java.time.ZonedDateTime

open class Core : androidx.lifecycle.ViewModel() {
    private var core: CoreFfi = CoreFfi()

    private val httpClient = HttpClient(CIO)

    var view: ViewModel? by mutableStateOf(null)
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

            is Effect.Time -> {
                val now = ZonedDateTime.now(ZoneOffset.UTC)
                val response = TimeResponse.Now(Instant(now.toEpochSecond().toULong(), now.nano.toUInt()))

                val effects = core.resolve(request.id, response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.Platform -> {
                val response = PlatformResponse(Build.BRAND + " " + Build.VERSION.RELEASE)

                val effects = core.resolve(request.id, response.bincodeSerialize())

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.KeyValue -> {}
        }
    }
}
