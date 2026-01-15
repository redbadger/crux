package com.crux.examples.counter

import io.ktor.client.HttpClient
import io.ktor.client.request.prepareGet
import io.ktor.client.statement.bodyAsChannel
import io.ktor.utils.io.readUTF8Line

@OptIn(ExperimentalUnsignedTypes::class)
suspend fun requestSse(
    client: HttpClient, request: SseRequest, callback: suspend (SseResponse) -> Unit
) {
    client.prepareGet(request.url).execute { response ->
        val channel = response.bodyAsChannel()
        while (!channel.isClosedForRead) {
            var chunk = channel.readUTF8Line() ?: break
            chunk += "\n\n"
            callback(SseResponse.Chunk(chunk.toByteArray().toUByteArray().toList()))
        }
        callback(SseResponse.Done)
    }
}
