package com.crux.examples.counter.core

import com.crux.examples.counter.SseRequest
import com.crux.examples.counter.SseResponse
import io.ktor.client.HttpClient
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.HttpTimeout
import io.ktor.client.plugins.logging.DEFAULT
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logger
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.prepareGet
import io.ktor.client.statement.bodyAsChannel
import io.ktor.utils.io.readLine

class SseClient {
    private val httpClient = HttpClient(OkHttp) {
        install(Logging) {
            logger = Logger.DEFAULT
            level = LogLevel.ALL
        }
        install(HttpTimeout) {
            requestTimeoutMillis = Long.MAX_VALUE
            connectTimeoutMillis = 15000
            socketTimeoutMillis = Long.MAX_VALUE
        }
    }

    @OptIn(ExperimentalUnsignedTypes::class)
    suspend fun request(
        request: SseRequest, callback: suspend (SseResponse) -> Unit
    ) {
        httpClient.prepareGet(request.url).execute { response ->
            val channel = response.bodyAsChannel()
            while (!channel.isClosedForRead) {
                var chunk = channel.readLine() ?: break
                chunk += "\n\n"
                callback(SseResponse.Chunk(chunk.toByteArray().toUByteArray().toList()))
            }
            callback(SseResponse.Done)
        }
    }
}
