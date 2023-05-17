package com.example.counter

import com.example.counter.shared_types.SseResponse
import io.ktor.client.*
import io.ktor.client.request.*
import io.ktor.client.statement.*
import io.ktor.utils.io.*
import io.ktor.utils.io.core.*

suspend fun sse(client: HttpClient, url: String, callback: suspend (SseResponse) -> Unit) {
    client.prepareGet(url).execute { response ->
        val channel = response.bodyAsChannel()
        while (!channel.isClosedForRead) {
            var chunk = channel.readUTF8Line() ?: break
            chunk += "\n\n"
            callback(SseResponse.Chunk(chunk.toByteArray().toList()))
        }
    }
}
