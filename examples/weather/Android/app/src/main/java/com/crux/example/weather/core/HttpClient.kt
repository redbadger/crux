package com.crux.example.weather.core

import com.crux.example.weather.HttpError
import com.crux.example.weather.HttpHeader
import com.crux.example.weather.HttpRequest
import com.crux.example.weather.HttpResponse
import com.crux.example.weather.HttpResult
import io.ktor.client.call.body
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.plugins.HttpRequestTimeoutException
import io.ktor.client.plugins.logging.DEFAULT
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logger
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.headers
import io.ktor.client.request.request
import io.ktor.client.request.setBody
import io.ktor.http.HttpMethod
import io.ktor.util.flattenEntries
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import kotlin.coroutines.cancellation.CancellationException
import io.ktor.client.HttpClient as KtorHttpClient

class HttpClient() {

    private val ktorHttpClient = KtorHttpClient(OkHttp) {
        install(Logging) {
            logger = Logger.DEFAULT
            level = LogLevel.ALL
        }
    }

    suspend fun request(request: HttpRequest): HttpResult {
        return try {
            val response = requestResponse(request)
            HttpResult.Ok(response)
        } catch (ce: CancellationException) {
            throw ce
        } catch (error: Throwable) {
            HttpResult.Err(toHttpError(error))
        }
    }

    private suspend fun requestResponse(request: HttpRequest): HttpResponse {
        val response = ktorHttpClient.request(request.url) {
            this.method = HttpMethod.parse(request.method)
            this.headers {
                for (header in request.headers) {
                    append(header.name, header.value)
                }
            }
            setBody(request.body)
        }
        val bytes: ByteArray = response.body()
        val headers = response.headers
            .flattenEntries()
            .map { HttpHeader(it.first, it.second) }
        return HttpResponse(response.status.value.toUShort(), headers, bytes)
    }

    private fun toHttpError(error: Throwable): HttpError {
        return when (error) {
            is HttpRequestTimeoutException, is SocketTimeoutException -> HttpError.Timeout
            is IllegalArgumentException -> HttpError.Url(error.message ?: "Invalid URL")
            is UnknownHostException -> HttpError.Io("Unknown host")
            else -> HttpError.Io(error.message ?: "HTTP request failed")
        }
    }
}
