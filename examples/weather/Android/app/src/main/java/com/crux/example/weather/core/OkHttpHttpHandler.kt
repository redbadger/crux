package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.HttpError
import com.crux.example.weather.HttpHandler
import com.crux.example.weather.HttpHeader
import com.crux.example.weather.HttpRequest
import com.crux.example.weather.HttpResponse
import com.crux.example.weather.HttpResult
import com.novi.serde.Bytes
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.InterruptedIOException
import java.net.UnknownHostException
import kotlin.coroutines.cancellation.CancellationException

/**
 * The app's [HttpHandler], over OkHttp.
 *
 * Every response is an answer, whatever its status. Only a transport failure
 * is an error, as the protocol says.
 */
class OkHttpHttpHandler(
    private val client: OkHttpClient,
) : HttpHandler {
    // ANCHOR: request
    override suspend fun request(operation: HttpRequest): HttpResult =
        withContext(Dispatchers.IO) {
            Log.d(TAG, "${operation.method} ${operation.url}")

            val url =
                operation.url.toHttpUrlOrNull()
                    ?: return@withContext HttpResult.Err(HttpError.Url("Not an absolute HTTP URL: ${operation.url}"))

            val method = operation.method.uppercase()
            val content = operation.body.content
            if (content.isNotEmpty() && method in BODY_FORBIDDEN_METHODS) {
                return@withContext HttpResult.Err(HttpError.Io("A $method request cannot have a body"))
            }

            try {
                val body =
                    when {
                        content.isNotEmpty() -> content.toRequestBody()
                        method in BODY_REQUIRED_METHODS -> ByteArray(0).toRequestBody()
                        else -> null
                    }

                val okRequest =
                    Request
                        .Builder()
                        .url(url)
                        .method(operation.method, body)
                        .apply { operation.headers.forEach { addHeader(it.name, it.value) } }
                        .build()

                client.newCall(okRequest).execute().use { response ->
                    val status = response.code.toUShort()
                    val headers = response.headers.toList().map { (name, value) -> HttpHeader(name, value) }
                    val responseBody = response.body?.bytes() ?: ByteArray(0)
                    Log.d(TAG, "${operation.method} ${operation.url} → $status")
                    HttpResult.Ok(HttpResponse(status, headers, Bytes(responseBody)))
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: InterruptedIOException) {
                // OkHttp reports each of its timeouts as an `InterruptedIOException`.
                Log.d(TAG, "timeout: ${operation.url}")
                HttpResult.Err(HttpError.Timeout)
            } catch (e: UnknownHostException) {
                Log.d(TAG, "unknown host: ${operation.url}")
                HttpResult.Err(HttpError.Io("Unknown host: ${e.message}"))
            } catch (e: Exception) {
                Log.w(TAG, "request failed for ${operation.url}: ${e.message}")
                HttpResult.Err(HttpError.Io(e.message ?: e.toString()))
            }
        }
    // ANCHOR_END: request

    companion object {
        private const val TAG = "OkHttpHttpHandler"
        private val BODY_REQUIRED_METHODS = setOf("POST", "PUT", "PATCH")
        private val BODY_FORBIDDEN_METHODS = setOf("GET", "HEAD")
    }
}
