package com.example.counter

import com.crux.example.counter.SseRequest
import com.crux.example.counter.SseResponse
import io.ktor.client.HttpClient
import io.ktor.client.request.prepareGet
import io.ktor.client.statement.bodyAsChannel
import io.ktor.utils.io.core.toByteArray
import io.ktor.utils.io.readUTF8Line

suspend fun requestSse(
    client: HttpClient, request: SseRequest, callback: suspend (SseResponse) -> Unit
) {
    client.prepareGet(request.url).execute { response ->
        val channel = response.bodyAsChannel()
        while (!channel.isClosedForRead) {
            var chunk = channel.readUTF8Line() ?: break
            chunk += "\n\n"
            callback(SseResponse.Chunk(chunk.toByteArray().toList()))
        }
        callback(SseResponse.Done())
    }
}
