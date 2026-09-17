import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException
import java.net.HttpURLConnection
import java.net.MalformedURLException
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URISyntaxException
import java.net.UnknownHostException

/// The shell side of `crux_http`, as one method per operation.
///
/// The method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```kotlin
/// class MyHandler : EffectHandler {
///     private val http = UrlConnectionHttpHandler.shared
///
///     override suspend fun http(operation: HttpRequest): HttpResult = http.request(operation)
/// }
/// ```
///
/// Implement it yourself — over OkHttp, Ktor, or whatever your app already
/// uses — to replace the implementation wholesale.
interface HttpHandler {
    /// Perform `operation` and answer with its result, exactly once.
    suspend fun request(operation: HttpRequest): HttpResult
}

/// An `HttpHandler` over the JDK's `HttpURLConnection`.
///
/// It is the client every JVM already has, which is why it is the one shipped;
/// a shell with an HTTP stack of its own implements [HttpHandler] over that
/// instead.
///
/// ## What is a failure
///
/// Every exchange the connection completes is an `HttpResult.Ok`, whatever the
/// status: 4xx and 5xx are answers, and the core decides what to make of them.
/// Only a transport failure — the request never reached a server, or no
/// response came back — is an `HttpResult.Err`, and then a URL that will not
/// parse is `HttpError.Url`, a `SocketTimeoutException` is `HttpError.Timeout`,
/// and everything else is `HttpError.Io`.
///
/// ## Methods
///
/// `HttpURLConnection` knows GET, POST, HEAD, OPTIONS, PUT, DELETE and TRACE,
/// and nothing else: a `PATCH` is answered with `HttpError.Io`. That is the
/// clearest reason to bring your own client.
///
/// @param configure runs against every connection before the request is sent,
///   for a timeout, a proxy, or anything else `HttpURLConnection` exposes.
class UrlConnectionHttpHandler(
    private val configure: (HttpURLConnection) -> Unit = {},
) : HttpHandler {
    override suspend fun request(operation: HttpRequest): HttpResult =
        withContext(Dispatchers.IO) {
            val url =
                try {
                    URI(operation.url).toURL()
                } catch (e: URISyntaxException) {
                    return@withContext HttpResult.Err(HttpError.Url(e.message ?: "could not parse URL"))
                } catch (e: MalformedURLException) {
                    return@withContext HttpResult.Err(HttpError.Url(e.message ?: "could not parse URL"))
                } catch (e: IllegalArgumentException) {
                    return@withContext HttpResult.Err(HttpError.Url(e.message ?: "could not parse URL"))
                }

            var connection: HttpURLConnection? = null
            try {
                connection = (url.openConnection() as HttpURLConnection).apply(configure)
                connection.requestMethod = operation.method.uppercase()
                // Header names and values cross the boundary as plain strings,
                // with no validation — the connection applies its own rules.
                for (header in operation.headers) {
                    connection.addRequestProperty(header.name, header.value)
                }

                val body = operation.body.content
                if (body.isNotEmpty() || connection.requestMethod in BODY_REQUIRED_METHODS) {
                    connection.doOutput = true
                    connection.setFixedLengthStreamingMode(body.size)
                    connection.outputStream.use { it.write(body) }
                }

                val status = connection.responseCode
                val headers = mutableListOf<HttpHeader>()
                for ((name, values) in connection.headerFields) {
                    // The null name is the status line, not a header.
                    if (name == null) continue
                    for (value in values) {
                        headers.add(HttpHeader(name, value))
                    }
                }
                // Below 400 the body is on the input stream, at or above it on
                // the error stream — and either may be absent.
                val stream =
                    if (status < HttpURLConnection.HTTP_BAD_REQUEST) {
                        connection.inputStream
                    } else {
                        connection.errorStream
                    }
                val responseBody = stream?.use { it.readBytes() } ?: ByteArray(0)

                // `com.novi.serde.Bytes` by its full name, and not imported:
                // the generated module's header already imports it above this
                // file's own imports, and Kotlin rejects the name twice.
                HttpResult.Ok(
                    HttpResponse(status.toUShort(), headers, com.novi.serde.Bytes(responseBody)),
                )
            } catch (e: SocketTimeoutException) {
                HttpResult.Err(HttpError.Timeout)
            } catch (e: UnknownHostException) {
                HttpResult.Err(HttpError.Io("unknown host: ${e.message}"))
            } catch (e: IOException) {
                HttpResult.Err(HttpError.Io(e.message ?: "IO error"))
            } finally {
                connection?.disconnect()
            }
        }

    companion object {
        /// The plain path: one handler, configured by nothing.
        val shared = UrlConnectionHttpHandler()

        /// `HttpURLConnection` sends no body unless it is told to, and a
        /// request with one of these methods and an empty body still has a
        /// body — an empty one.
        private val BODY_REQUIRED_METHODS = setOf("POST", "PUT", "PATCH")
    }
}
