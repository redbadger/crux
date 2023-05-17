package com.example.counter

import com.example.counter.shared_types.HttpHeader
import com.example.counter.shared_types.HttpResponse
import io.ktor.client.*
import io.ktor.client.call.*
import io.ktor.client.request.*
import io.ktor.http.*

suspend fun http(
    client: HttpClient,
    method: HttpMethod,
    url: String,
    headers: List<HttpHeader>
): HttpResponse {
    val response = client.request(url) {
        this.method = method
        this.headers {
            for (header in headers) {
                append(header.name, header.value)
            }
        }
    }
    val bytes: ByteArray = response.body()
    return HttpResponse(response.status.value.toShort(), bytes.toList())
}
