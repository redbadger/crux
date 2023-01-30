package com.example.counter

import com.example.counter.shared_types.HttpResponse
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*

suspend fun http(
    client: HttpClient,
    method: HttpMethod,
    url: String
): HttpResponse {
    val response = client.request(url) {
        this.method = method
    }
    val bytes: ByteArray = response.body()
    return HttpResponse(response.status.value.toShort(), bytes.toList())
}