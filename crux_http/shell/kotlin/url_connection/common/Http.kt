import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.IOException
import java.net.HttpCookie
import java.net.HttpURLConnection
import java.net.MalformedURLException
import java.net.SocketTimeoutException
import java.net.URI
import java.net.URISyntaxException
import java.net.URL
import java.net.UnknownHostException
import java.util.concurrent.ConcurrentHashMap

/**
 * The shell side of `crux_http`, as one method per operation.
 *
 * The method's shape is the generated `EffectHandler`'s, so an app's handler
 * holds an instance and delegates in one line:
 *
 * ```kotlin
 * class MyHandler : EffectHandler {
 *     private val http = UrlConnectionHttpHandler()
 *
 *     override suspend fun http(operation: HttpRequest): HttpResult = http.request(operation)
 * }
 * ```
 *
 * Implement it yourself — over OkHttp, Ktor, or whatever your app already
 * uses — to replace the implementation wholesale.
 */
interface HttpHandler {
    /** Perform `operation` and answer with its result, exactly once. */
    suspend fun request(operation: HttpRequest): HttpResult
}

/**
 * The cookies a handler carries from one request to the next.
 *
 * Implement it to persist them — to a file, a database, a platform store —
 * or to scope them more narrowly than [InMemoryCookieJar] does.
 */
interface CookieJar {
    /** The `Cookie` header value for a request to [url], or `null` to send none. */
    fun headerFor(url: URI): String?

    /** Take in the `Set-Cookie` values a response from [url] carried. */
    fun absorb(
        url: URI,
        setCookie: List<String>,
    )
}

/**
 * A [CookieJar] in memory, for as long as the handler lives.
 *
 * Enough for a session cookie, which is what most apps want one for. Cookies
 * are matched the way RFC 6265 says: host or domain, path prefix, and `Secure`
 * only over `https`. A cookie that has expired — including the `Max-Age=0` a
 * server deletes one with — is dropped rather than replayed.
 *
 * ## Which `Domain` a server can set
 *
 * The jar refuses a cookie whose `Domain` does not domain-match the host that
 * set it (RFC 6265, section 5.3). It also refuses a `Domain` with one label,
 * such as `com`, and a `Domain` that is not the IP address that set it. The
 * jar has no public suffix list, so it accepts `Domain=co.uk` from
 * `example.co.uk`. If that is a risk for your app, implement [CookieJar] over
 * a library that has the list.
 *
 * ## Why this is not `java.net.CookieManager`
 *
 * Because that one does not work here, and fails in a way that reads as a
 * login bug rather than a transport one. `HttpURLConnection` consults the
 * process-wide `CookieHandler`, so installing the JDK's `CookieManager` as the
 * default is the obvious move — but a cookie carrying `Max-Age` is parsed as
 * RFC 2965 version 1, and `CookieManager` then *writes* it back that way:
 *
 * ```text
 * Cookie: $Version="1"; sid="abc";$Path="/";$Domain="example.com"
 * ```
 *
 * The value is quoted and the `$` pseudo-attributes are junk, so a server
 * following RFC 6265 — which is all of them — reads no cookie at all. Almost
 * every server sets `Max-Age`, so this is the common case rather than a corner.
 * `HttpCookie`'s *parser* is fine, which is why [absorb] still uses it; only
 * the writing is done here.
 */
class InMemoryCookieJar : CookieJar {
    /**
     * RFC 6265 identifies a cookie by its name, domain and path. A new cookie
     * with the same three replaces the old one, with or without `Domain`.
     */
    private data class Key(
        val name: String,
        val domain: String,
        val path: String,
    )

    /**
     * `hostOnly` is kept because it changes what [domainMatches] means: a
     * cookie set without `Domain` belongs to exactly the host that set it, and
     * must not reach a subdomain.
     */
    private class Stored(
        val cookie: HttpCookie,
        val hostOnly: Boolean,
    )

    private val cookies = ConcurrentHashMap<Key, Stored>()

    override fun headerFor(url: URI): String? {
        val host = url.host ?: return null
        val secure = url.scheme.equals("https", ignoreCase = true)
        val path = url.path.ifEmpty { "/" }
        val matching =
            cookies.entries
                .filter { (key, stored) ->
                    !stored.cookie.hasExpired() &&
                        (!stored.cookie.secure || secure) &&
                        domainMatches(key.domain, host, stored.hostOnly) &&
                        pathMatches(key.path, path)
                }
                // RFC 6265 orders by descending path length, so the most
                // specific cookie is the one a server reading only the first
                // sees.
                .sortedByDescending { (key, _) -> key.path.length }
                .map { (key, stored) -> "${key.name}=${stored.cookie.value}" }
        return matching.takeIf { it.isNotEmpty() }?.joinToString("; ")
    }

    override fun absorb(
        url: URI,
        setCookie: List<String>,
    ) {
        val host = url.host?.lowercase() ?: return
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
                val declared = cookie.domain?.removePrefix(".")?.lowercase()?.takeIf { it.isNotEmpty() }
                if (declared != null && !mayDeclare(declared, host)) continue
                // No `Domain` means host-only: the cookie goes back to exactly
                // the host that set it and no subdomain of it.
                val key = Key(cookie.name, declared ?: host, cookie.path ?: defaultPath(url))
                if (cookie.hasExpired()) {
                    cookies.remove(key)
                } else {
                    cookies[key] = Stored(cookie, hostOnly = declared == null)
                }
            }
        }
    }

    /** Whether [host] can set a cookie with `Domain=`[domain]. */
    private fun mayDeclare(
        domain: String,
        host: String,
    ): Boolean =
        when {
            !domainMatches(domain, host, hostOnly = false) -> false
            // A domain with one label is a top-level domain, which no host owns.
            '.' !in domain -> false
            // `1.2.3.4` ends with `.2.3.4`, but an address is not a domain.
            isIpLiteral(host) -> host == domain
            else -> true
        }

    private fun isIpLiteral(host: String): Boolean = host.startsWith("[") || IPV4.matches(host)

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

    /** RFC 6265's default-path: the directory the request was made in. */
    private fun defaultPath(url: URI): String {
        val path = url.path
        if (path.isNullOrEmpty() || !path.startsWith("/")) return "/"
        val lastSlash = path.lastIndexOf('/')
        return if (lastSlash <= 0) "/" else path.substring(0, lastSlash)
    }

    private companion object {
        val IPV4 = Regex("""\d{1,3}(\.\d{1,3}){3}""")
    }
}

/**
 * An `HttpHandler` over the JDK's `HttpURLConnection`.
 *
 * It is the client every JVM already has, which is why it is the one shipped;
 * a shell with an HTTP stack of its own implements [HttpHandler] over that
 * instead.
 *
 * ## What is a failure
 *
 * Every exchange the connection completes is an `HttpResult.Ok`, whatever the
 * status: 4xx and 5xx are answers, and the core decides what to make of them.
 * Only a transport failure — the request never reached a server, or no
 * response came back — is an `HttpResult.Err`, and then a URL that will not
 * parse is `HttpError.Url`, a `SocketTimeoutException` is `HttpError.Timeout`,
 * and everything else is `HttpError.Io`.
 *
 * The core builds `operation.url` with `url::Url`, so the URL is absolute. A
 * relative URL, or a URL whose scheme is not `http` or `https`, is
 * `HttpError.Url`. A `GET` or `HEAD` with a body is `HttpError.Io`, and the
 * handler does not send it.
 *
 * ## Methods
 *
 * On the desktop JDK, `HttpURLConnection` knows GET, POST, HEAD, OPTIONS,
 * PUT, DELETE and TRACE, and answers any other method, `PATCH` included, with
 * `HttpError.Io`. Android's implementation also knows `PATCH`. If your app
 * needs another method, bring your own client.
 *
 * ## Redirects
 *
 * The handler follows redirects itself, so that the cookie jar sees each
 * response on the way. It sets `instanceFollowRedirects` to `false` after
 * `configure` runs. The rules are these:
 *
 * - It follows at most 20 redirects. After that, the answer is
 *   `HttpError.Io`.
 * - A 301, 302 or 303 becomes a `GET` without a body, unless the request is
 *   a `HEAD`. A 307 or 308 keeps the method and the body.
 * - It does not follow a redirect from `https` to `http`, or a redirect
 *   without a usable `Location`. The core gets that 3xx response.
 * - When a redirect leaves the origin of the request, the handler removes
 *   the operation's own `Cookie` and `Authorization` headers.
 *
 * ## Cookies
 *
 * `HttpURLConnection` keeps no cookies of its own, so the handler keeps them,
 * by default, in a jar of its own. A session then survives between requests,
 * as it does under `URLSession` on Apple and `HttpClient` on .NET. One
 * handler is one jar: share the instance to share the session. Pass
 * `cookies = null` for a handler that keeps none, or a [CookieJar] of your own
 * to persist them. **Do not reach for `java.net.CookieManager`** —
 * [InMemoryCookieJar] documents why it cannot work.
 *
 * A `Cookie` header the operation already carries is left alone: a core that
 * has said what it wants is not asking for help.
 *
 * @param cookies the jar carried between requests, or `null` to keep none.
 * @param configure runs against every connection before the request is sent,
 *   for a timeout, a proxy, or anything else `HttpURLConnection` exposes.
 */
class UrlConnectionHttpHandler(
    private val cookies: CookieJar? = InMemoryCookieJar(),
    private val configure: (HttpURLConnection) -> Unit = {},
) : HttpHandler {
    /** One request of the exchange: the first, or one a redirect asked for. */
    private class Hop(
        val uri: URI,
        val url: URL,
        val method: String,
        val headers: List<HttpHeader>,
        val body: ByteArray,
    )

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
                urlOf(uri)
                    ?: return@withContext HttpResult.Err(HttpError.Url("could not parse URL: ${operation.url}"))

            val method = operation.method.uppercase()
            val body = operation.body.content
            // `HttpURLConnection` silently turns a GET with a body into a POST.
            if (body.isNotEmpty() && method in BODILESS_METHODS) {
                return@withContext HttpResult.Err(HttpError.Io("$method request cannot carry a body"))
            }

            try {
                exchange(Hop(uri, url, method, operation.headers, body))
            } catch (e: SocketTimeoutException) {
                HttpResult.Err(HttpError.Timeout)
            } catch (e: UnknownHostException) {
                HttpResult.Err(HttpError.Io("unknown host: ${e.message}"))
            } catch (e: IOException) {
                HttpResult.Err(HttpError.Io(e.message ?: "IO error"))
            } catch (e: IllegalArgumentException) {
                // A header name or value that the connection refuses, for
                // example one with a line break in it.
                HttpResult.Err(HttpError.Io(e.message ?: "invalid request"))
            }
        }

    /**
     * Sends [first], then each request a redirect asks for, and answers with
     * the response that is not followed.
     */
    private fun exchange(first: Hop): HttpResult {
        var hop = first
        repeat(MAX_REDIRECTS + 1) {
            val connection =
                hop.url.openConnection() as? HttpURLConnection
                    ?: throw IOException("not an HTTP connection: ${hop.url}")
            try {
                send(connection, hop)

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
                // reading the response then fails. Each hop's cookies belong
                // to that hop's URI.
                if (setCookie.isNotEmpty()) cookies?.absorb(hop.uri, setCookie)

                val next =
                    if (status in REDIRECT_STATUSES) {
                        redirect(hop, status, connection.getHeaderField("Location"), first.uri)
                    } else {
                        null
                    }
                if (next == null) {
                    // `com.novi.serde.Bytes` by its full name, and not
                    // imported: the generated module's header already imports
                    // it above this file's own imports, and Kotlin rejects the
                    // name twice.
                    return HttpResult.Ok(
                        HttpResponse(status.toUShort(), headers, com.novi.serde.Bytes(readBody(connection, status))),
                    )
                }
                hop = next
            } finally {
                connection.disconnect()
            }
        }
        return HttpResult.Err(HttpError.Io("too many redirects"))
    }

    private fun send(
        connection: HttpURLConnection,
        hop: Hop,
    ) {
        connection.apply(configure)
        connection.instanceFollowRedirects = false
        connection.requestMethod = hop.method
        // Header names and values cross the boundary as plain strings,
        // with no validation — the connection applies its own rules.
        for (header in hop.headers) {
            connection.addRequestProperty(header.name, header.value)
        }
        if (hop.headers.none { it.name.equals("cookie", ignoreCase = true) }) {
            cookies?.headerFor(hop.uri)?.let { connection.addRequestProperty("Cookie", it) }
        }

        if (hop.body.isNotEmpty() || hop.method in BODY_REQUIRED_METHODS) {
            connection.doOutput = true
            connection.setFixedLengthStreamingMode(hop.body.size)
            connection.outputStream.use { it.write(hop.body) }
        }
    }

    /**
     * The request a redirect from [from] asks for, or `null` if the handler
     * does not follow it.
     */
    private fun redirect(
        from: Hop,
        status: Int,
        location: String?,
        origin: URI,
    ): Hop? {
        if (location == null) return null
        val target = resolve(from.uri, location) ?: return null
        // Browsers do not follow a redirect to a less secure scheme.
        if (from.uri.scheme.equals("https", ignoreCase = true) && target.scheme.equals("http", ignoreCase = true)) {
            return null
        }
        val url = urlOf(target) ?: return null

        val asGet = status in GET_AFTER_STATUSES && from.method != "HEAD"
        val leavesOrigin = !sameOrigin(origin, target)
        val headers =
            from.headers.filterNot { header ->
                val name = header.name.lowercase()
                (asGet && name in CONTENT_HEADERS) || (leavesOrigin && name in CREDENTIAL_HEADERS)
            }
        return if (asGet) {
            Hop(target, url, "GET", headers, ByteArray(0))
        } else {
            Hop(target, url, from.method, headers, from.body)
        }
    }

    private fun readBody(
        connection: HttpURLConnection,
        status: Int,
    ): ByteArray {
        // Below 400 the body is on the input stream, at or above it on
        // the error stream — and either may be absent.
        val stream =
            if (status < HttpURLConnection.HTTP_BAD_REQUEST) {
                connection.inputStream
            } else {
                connection.errorStream
            }
        return stream?.use { it.readBytes() } ?: ByteArray(0)
    }

    companion object {
        /**
         * `HttpURLConnection` sends no body unless it is told to, and a
         * request with one of these methods and an empty body still has a
         * body — an empty one.
         */
        private val BODY_REQUIRED_METHODS = setOf("POST", "PUT", "PATCH")

        /** The methods that cannot carry a body. */
        private val BODILESS_METHODS = setOf("GET", "HEAD")

        /** The most redirects one request follows, as in Chromium and Firefox. */
        private const val MAX_REDIRECTS = 20

        private val REDIRECT_STATUSES = setOf(301, 302, 303, 307, 308)

        /** The redirects after which a request becomes a `GET` without a body. */
        private val GET_AFTER_STATUSES = setOf(301, 302, 303)

        /** The headers that describe a body, removed when the body is. */
        private val CONTENT_HEADERS =
            setOf("content-type", "content-length", "content-encoding", "content-language", "content-location")

        /** The headers that only the origin of the request receives. */
        private val CREDENTIAL_HEADERS = setOf("cookie", "authorization")

        private fun urlOf(uri: URI): URL? {
            val scheme = uri.scheme?.lowercase()
            if (scheme != "http" && scheme != "https") return null
            return try {
                uri.toURL()
            } catch (e: MalformedURLException) {
                null
            } catch (e: IllegalArgumentException) {
                null
            }
        }

        /** [location] resolved against [base], or `null` if it does not parse. */
        private fun resolve(
            base: URI,
            location: String,
        ): URI? =
            try {
                // `URI.resolve` drops the slash between an empty path and a
                // relative reference, so give the base a path first.
                val directory = if (base.rawPath.isNullOrEmpty()) base.resolve("/") else base
                directory.resolve(location)
            } catch (e: IllegalArgumentException) {
                null
            }

        private fun sameOrigin(
            a: URI,
            b: URI,
        ): Boolean =
            a.scheme.equals(b.scheme, ignoreCase = true) &&
                (a.host ?: a.rawAuthority).equals(b.host ?: b.rawAuthority, ignoreCase = true) &&
                portOf(a) == portOf(b)

        private fun portOf(uri: URI): Int =
            when {
                uri.port != -1 -> uri.port
                uri.scheme.equals("https", ignoreCase = true) -> 443
                else -> 80
            }
    }
}
