import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException
import java.net.HttpCookie
import java.net.HttpURLConnection
import java.net.MalformedURLException
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URISyntaxException
import java.net.UnknownHostException
import java.util.concurrent.ConcurrentHashMap

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

/// The cookies a handler carries from one request to the next.
///
/// Implement it to persist them — to a file, a database, a platform store —
/// or to scope them more narrowly than [InMemoryCookieJar] does.
interface CookieJar {
    /// The `Cookie` header value for a request to [url], or `null` to send none.
    fun headerFor(url: URI): String?

    /// Take in the `Set-Cookie` values a response from [url] carried.
    fun absorb(
        url: URI,
        setCookie: List<String>,
    )
}

/// A [CookieJar] in memory, for as long as the handler lives.
///
/// Enough for a session cookie, which is what most apps want one for. Cookies
/// are matched the way RFC 6265 says: host or domain, path prefix, and `Secure`
/// only over `https`. A cookie that has expired — including the `Max-Age=0` a
/// server deletes one with — is dropped rather than replayed.
///
/// ## Why this is not `java.net.CookieManager`
///
/// Because that one does not work here, and fails in a way that reads as a
/// login bug rather than a transport one. `HttpURLConnection` consults the
/// process-wide `CookieHandler`, so installing the JDK's `CookieManager` as the
/// default is the obvious move — but a cookie carrying `Max-Age` is parsed as
/// RFC 2965 version 1, and `CookieManager` then *writes* it back that way:
///
/// ```text
/// Cookie: $Version="1"; sid="abc";$Path="/";$Domain="example.com"
/// ```
///
/// The value is quoted and the `$` pseudo-attributes are junk, so a server
/// following RFC 6265 — which is all of them — reads no cookie at all. Almost
/// every server sets `Max-Age`, so this is the common case rather than a corner.
/// `HttpCookie`'s *parser* is fine, which is why [absorb] still uses it; only
/// the writing is done here.
class InMemoryCookieJar : CookieJar {
    /// `hostOnly` is carried because it changes what [domainMatches] means: a
    /// cookie set without `Domain` belongs to exactly the host that set it, and
    /// must not reach a subdomain.
    private data class Key(
        val name: String,
        val domain: String,
        val path: String,
        val hostOnly: Boolean,
    )

    private val cookies = ConcurrentHashMap<Key, HttpCookie>()

    override fun headerFor(url: URI): String? {
        val host = url.host ?: return null
        val secure = url.scheme.equals("https", ignoreCase = true)
        val path = url.path.ifEmpty { "/" }
        val matching =
            cookies.entries
                .filter { (key, cookie) ->
                    !cookie.hasExpired() &&
                        (!cookie.secure || secure) &&
                        domainMatches(key.domain, host, key.hostOnly) &&
                        pathMatches(key.path, path)
                }
                // RFC 6265 orders by descending path length, so the most
                // specific cookie is the one a server reading only the first
                // sees.
                .sortedByDescending { (key, _) -> key.path.length }
                .map { (key, cookie) -> "${key.name}=${cookie.value}" }
        return matching.takeIf { it.isNotEmpty() }?.joinToString("; ")
    }

    override fun absorb(
        url: URI,
        setCookie: List<String>,
    ) {
        val host = url.host ?: return
        for (header in setCookie) {
            // A malformed `Set-Cookie` is the server's problem, and dropping
            // the exchange over it would be worse than ignoring the cookie.
            val parsed =
                try {
                    HttpCookie.parse(header)
                } catch (e: IllegalArgumentException) {
                    continue
                }
            for (cookie in parsed) {
                // No `Domain` means host-only: the cookie goes back to exactly
                // the host that set it and no subdomain of it.
                val key =
                    Key(
                        cookie.name,
                        cookie.domain?.removePrefix(".")?.takeIf { it.isNotEmpty() } ?: host,
                        cookie.path ?: defaultPath(url),
                        hostOnly = cookie.domain.isNullOrEmpty(),
                    )
                if (cookie.hasExpired()) cookies.remove(key) else cookies[key] = cookie
            }
        }
    }

    private fun domainMatches(
        domain: String,
        host: String,
        hostOnly: Boolean,
    ): Boolean =
        when {
            host.equals(domain, ignoreCase = true) -> true
            hostOnly -> false
            else -> host.endsWith(".$domain", ignoreCase = true)
        }

    private fun pathMatches(
        scope: String,
        path: String,
    ): Boolean = path == scope || path.startsWith(if (scope.endsWith("/")) scope else "$scope/")

    /// RFC 6265's default-path: the directory the request was made in.
    private fun defaultPath(url: URI): String {
        val path = url.path
        if (path.isNullOrEmpty() || !path.startsWith("/")) return "/"
        val lastSlash = path.lastIndexOf('/')
        return if (lastSlash <= 0) "/" else path.substring(0, lastSlash)
    }
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
/// ## Cookies
///
/// Kept, by default, in a jar of this handler's own — so a session survives
/// between requests, as it does under `URLSession` on Apple and `HttpClient`
/// on .NET. One handler is one jar: share the instance to share the session.
/// Pass `cookies = null` for a handler that keeps none, or a [CookieJar] of
/// your own to persist them. **Do not reach for `java.net.CookieManager`** —
/// [InMemoryCookieJar] documents why it cannot work.
///
/// A `Cookie` header the operation already carries is left alone: a core that
/// has said what it wants is not asking for help.
///
/// @param cookies the jar carried between requests, or `null` to keep none.
/// @param configure runs against every connection before the request is sent,
///   for a timeout, a proxy, or anything else `HttpURLConnection` exposes.
class UrlConnectionHttpHandler(
    private val cookies: CookieJar? = InMemoryCookieJar(),
    private val configure: (HttpURLConnection) -> Unit = {},
) : HttpHandler {
    override suspend fun request(operation: HttpRequest): HttpResult =
        withContext(Dispatchers.IO) {
            val uri =
                try {
                    URI(operation.url)
                } catch (e: URISyntaxException) {
                    return@withContext HttpResult.Err(HttpError.Url(e.message ?: "could not parse URL"))
                } catch (e: IllegalArgumentException) {
                    return@withContext HttpResult.Err(HttpError.Url(e.message ?: "could not parse URL"))
                }
            val url =
                try {
                    uri.toURL()
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
                if (operation.headers.none { it.name.equals("cookie", ignoreCase = true) }) {
                    cookies?.headerFor(uri)?.let { connection.addRequestProperty("Cookie", it) }
                }

                val body = operation.body.content
                if (body.isNotEmpty() || connection.requestMethod in BODY_REQUIRED_METHODS) {
                    connection.doOutput = true
                    connection.setFixedLengthStreamingMode(body.size)
                    connection.outputStream.use { it.write(body) }
                }

                val status = connection.responseCode
                val headers = mutableListOf<HttpHeader>()
                val setCookie = mutableListOf<String>()
                for ((name, values) in connection.headerFields) {
                    // The null name is the status line, not a header.
                    if (name == null) continue
                    for (value in values) {
                        headers.add(HttpHeader(name, value))
                    }
                    if (name.equals("set-cookie", ignoreCase = true)) setCookie.addAll(values)
                }
                // Before the body is read, so that a cookie is kept even if
                // reading the response then fails.
                if (setCookie.isNotEmpty()) cookies?.absorb(uri, setCookie)
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
        /// The plain path: one handler, configured by nothing — and so one
        /// cookie jar, shared by everything that reaches for it.
        val shared = UrlConnectionHttpHandler()

        /// `HttpURLConnection` sends no body unless it is told to, and a
        /// request with one of these methods and an empty body still has a
        /// body — an empty one.
        private val BODY_REQUIRED_METHODS = setOf("POST", "PUT", "PATCH")
    }
}
