package com.crux.example.weather.core

import android.util.Log
import com.crux.example.weather.HttpError
import com.crux.example.weather.HttpHeader
import com.crux.example.weather.HttpRequest
import com.crux.example.weather.HttpResponse
import com.crux.example.weather.HttpResult
import com.novi.serde.Bytes
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.net.SocketTimeoutException
import java.net.UnknownHostException
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class HttpHandler
    @Inject
    constructor(
        private val client: OkHttpClient,
    ) {
        // ANCHOR: request
        suspend fun request(op: HttpRequest): HttpResult =
            withContext(Dispatchers.IO) {
                Log.d(TAG, "${op.method} ${op.url}")
                try {
                    val body =
                        when {
                            op.body.content.isNotEmpty() ->
                                op.body.content.toUByteArray().toByteArray().toRequestBody()
                            op.method.uppercase() in BODY_REQUIRED_METHODS -> ByteArray(0).toRequestBody()
                            else -> null
                        }

                    val okRequest =
                        Request
                            .Builder()
                            .url(op.url)
                            .method(op.method, body)
                            .apply { op.headers.forEach { addHeader(it.name, it.value) } }
                            .build()

                    client.newCall(okRequest).execute().use { response ->
                        val status = response.code.toUShort()
                        val headers = response.headers.toList().map { (name, value) -> HttpHeader(name, value) }
                        val responseBody = response.body?.bytes() ?: ByteArray(0)
                        Log.d(TAG, "${op.method} ${op.url} → $status")
                        HttpResult.Ok(HttpResponse(status, headers, Bytes(responseBody)))
                    }
                } catch (e: SocketTimeoutException) {
                    Log.d(TAG, "timeout: ${op.url}")
                    HttpResult.Err(HttpError.Timeout)
                } catch (e: UnknownHostException) {
                    Log.d(TAG, "unknown host: ${op.url}")
                    HttpResult.Err(HttpError.Io("Unknown host: ${e.message}"))
                } catch (e: IllegalArgumentException) {
                    Log.w(TAG, "invalid URL ${op.url}: ${e.message}")
                    HttpResult.Err(HttpError.Url(e.message ?: "Invalid URL"))
                } catch (e: Exception) {
                    Log.w(TAG, "request failed for ${op.url}: ${e.message}")
                    HttpResult.Err(HttpError.Io(e.message ?: "IO error"))
                }
            }
        // ANCHOR_END: request

        companion object {
            private const val TAG = "HttpHandler"
            private val BODY_REQUIRED_METHODS = setOf("POST", "PUT", "PATCH")
        }
    }
