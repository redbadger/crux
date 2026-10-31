import java.net.HttpCookie
import java.net.URI
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
