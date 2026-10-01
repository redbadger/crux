package com.crux.examples.counter.http

import com.crux.examples.counter.HttpError
import com.crux.examples.counter.HttpHeader
import com.crux.examples.counter.HttpRequest
import com.crux.examples.counter.HttpResponse
import com.crux.examples.counter.HttpResult
import com.novi.serde.Bytes
import io.ktor.client.call.body
import io.ktor.client.engine.okhttp.OkHttp
import io.ktor.client.network.sockets.ConnectTimeoutException
import io.ktor.client.plugins.HttpRequestTimeoutException
import io.ktor.client.plugins.HttpTimeout
import io.ktor.client.plugins.logging.DEFAULT
import io.ktor.client.plugins.logging.LogLevel
import io.ktor.client.plugins.logging.Logger
import io.ktor.client.plugins.logging.Logging
import io.ktor.client.request.headers
import io.ktor.client.request.request
import io.ktor.client.request.setBody
import io.ktor.http.HttpMethod
import io.ktor.util.flattenEntries
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URISyntaxException
import java.net.UnknownHostException
import kotlin.coroutines.cancellation.CancellationException
import io.ktor.client.HttpClient as KtorHttpClient

/**
 * Sends an [HttpRequest] over Ktor with the OkHttp engine.
 *
 * Every response is an answer, whatever its status. Only a transport failure
 * is an error, as the doc of [HttpRequest] says.
 */
class KtorHttpHandler {
    private val client =
        KtorHttpClient(OkHttp) {
            install(Logging) {
                logger = Logger.DEFAULT
                level = LogLevel.ALL
            }
            install(HttpTimeout) {
                requestTimeoutMillis = 30000
                connectTimeoutMillis = 15000
                socketTimeoutMillis = 15000
            }
        }

    suspend fun request(operation: HttpRequest): HttpResult =
        withContext(Dispatchers.IO) {
            if (!isAbsolute(operation.url)) {
                return@withContext HttpResult.Err(HttpError.Url("Not an absolute URL: ${operation.url}"))
            }

            val method = operation.method.uppercase()
            val content = operation.body.content
            if (content.isNotEmpty() && method in BODY_FORBIDDEN_METHODS) {
                return@withContext HttpResult.Err(HttpError.Io("A $method request cannot have a body"))
            }

            try {
                val response =
                    client.request(operation.url) {
                        this.method = HttpMethod.parse(method)
                        headers {
                            for (header in operation.headers) {
                                append(header.name, header.value)
                            }
                        }
                        if (content.isNotEmpty()) {
                            setBody(content)
                        }
                    }

                val bytes: ByteArray = response.body()
                val headers =
                    response.headers
                        .flattenEntries()
                        .map { HttpHeader(it.first, it.second) }
                HttpResult.Ok(HttpResponse(response.status.value.toUShort(), headers, Bytes(bytes)))
            } catch (e: CancellationException) {
                throw e
            } catch (e: HttpRequestTimeoutException) {
                HttpResult.Err(HttpError.Timeout)
            } catch (e: ConnectTimeoutException) {
                HttpResult.Err(HttpError.Timeout)
            } catch (e: SocketTimeoutException) {
                HttpResult.Err(HttpError.Timeout)
            } catch (e: UnknownHostException) {
                HttpResult.Err(HttpError.Io("Unknown host: ${e.message}"))
            } catch (e: Exception) {
                HttpResult.Err(HttpError.Io(e.message ?: e.toString()))
            }
        }

    private fun isAbsolute(url: String): Boolean =
        try {
            URI(url).isAbsolute
        } catch (e: URISyntaxException) {
            false
        }

    companion object {
        private val BODY_FORBIDDEN_METHODS = setOf("GET", "HEAD")
    }
}
