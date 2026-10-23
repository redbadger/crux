import com.example.shared.HttpError
import com.example.shared.HttpHeader
import com.example.shared.HttpRequest
import com.example.shared.HttpResult
import com.example.shared.InMemoryCookieJar
import com.example.shared.UrlConnectionHttpHandler
import com.novi.serde.Bytes
import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import kotlinx.coroutines.runBlocking
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.URI
import kotlin.system.exitProcess

/**
 * Runs `InMemoryCookieJar` through RFC 6265's matching rules and prints
 * `HARNESS OK` when every answer was the one a server would expect.
 *
 * The jar, not the handler: what it puts in a `Cookie` header is all
 * behaviour, and none of it is a compile error. The case that matters most is
 * the first — a cookie carrying `Max-Age`, which `java.net.CookieManager`
 * would write back in RFC 2965 form (`$Version="1"; sid="abc";$Path="/"`) that
 * no server reads. That is why this jar exists, so that is what is asserted.
 *
 * Then `UrlConnectionHttpHandler`, against a server in this process. The
 * handler follows each redirect itself, so the harness checks the method,
 * body, cookies and credentials of each hop the server sees.
 */

// Every mismatch, so that one run reports all of them rather than the first.
val failures = mutableListOf<String>()

fun expect(
    what: String,
    satisfied: Boolean,
) {
    if (!satisfied) failures.add(what)
}

fun <T> expect(
    what: String,
    actual: T,
    expected: T,
) {
    if (actual != expected) failures.add("$what: expected $expected, got $actual")
}

fun main() {
    val origin = URI("https://example.com/api/me")

    // A session cookie exactly as a server sets one, `Max-Age` and all.
    val session = InMemoryCookieJar()
    session.absorb(origin, listOf("sid=abc; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=28800"))
    expect("a Max-Age cookie goes back as name=value", session.headerFor(origin), "sid=abc")

    // The whole point, stated as the thing that must never appear.
    val header = session.headerFor(origin) ?: ""
    expect("no RFC 2965 \$Version prefix", !header.contains("\$Version"))
    expect("no RFC 2965 \$Path attribute", !header.contains("\$Path"))
    expect("the value is not quoted", !header.contains("\""))

    // Host-only: no `Domain` means exactly the host that set it.
    expect("a host-only cookie is not sent elsewhere", session.headerFor(URI("https://other.com/api/me")), null)
    expect("a host-only cookie is not sent to a subdomain", session.headerFor(URI("https://a.example.com/")), null)

    // `Secure` is honoured, or a production session goes out in clear.
    expect("a Secure cookie is not sent over http", session.headerFor(URI("http://example.com/api/me")), null)

    // A `Domain` cookie does reach subdomains.
    val wide = InMemoryCookieJar()
    wide.absorb(origin, listOf("tenant=eu; Domain=example.com; Path=/"))
    expect("a Domain cookie reaches a subdomain", wide.headerFor(URI("https://a.example.com/")), "tenant=eu")

    // Path scope, including the boundary a naive prefix check gets wrong.
    val scoped = InMemoryCookieJar()
    scoped.absorb(origin, listOf("deep=1; Path=/api"))
    expect("a path-scoped cookie is sent within its path", scoped.headerFor(URI("https://example.com/api/me")), "deep=1")
    expect("a path-scoped cookie is sent at its path", scoped.headerFor(URI("https://example.com/api")), "deep=1")
    expect("a path-scoped cookie is not sent to a sibling", scoped.headerFor(URI("https://example.com/apiary")), null)

    // `Max-Age=0` is how a server signs you out.
    session.absorb(origin, listOf("sid=abc; Path=/; Max-Age=0"))
    expect("a Max-Age=0 cookie is forgotten", session.headerFor(origin), null)

    // Two cookies travel in one header, most specific path first.
    val several = InMemoryCookieJar()
    several.absorb(origin, listOf("root=1; Path=/", "inner=2; Path=/api"))
    expect("several cookies share one header, longest path first", several.headerFor(origin), "inner=2; root=1")

    // A malformed Set-Cookie is the server's problem, not an exception here.
    val tolerant = InMemoryCookieJar()
    tolerant.absorb(origin, listOf("=nonsense=", "ok=1; Path=/"))
    expect("a malformed Set-Cookie does not lose the good one", tolerant.headerFor(origin), "ok=1")

    // A host can set a cookie only for a domain that it is in, and not for a
    // top-level domain.
    val foreign = InMemoryCookieJar()
    foreign.absorb(URI("https://a.example/"), listOf("x=1; Domain=b.example; Path=/"))
    expect("a Domain that the host is not in is refused", foreign.headerFor(URI("https://b.example/")), null)
    val topLevel = InMemoryCookieJar()
    topLevel.absorb(origin, listOf("x=1; Domain=com; Path=/"))
    expect("a Domain with one label is refused", topLevel.headerFor(URI("https://other.com/")), null)

    // RFC 6265 identifies a cookie by name, domain and path, so a host-only
    // cookie and a Domain cookie with the same three are one cookie.
    val replaced = InMemoryCookieJar()
    replaced.absorb(origin, listOf("sid=a; Path=/"))
    replaced.absorb(origin, listOf("sid=b; Domain=example.com; Path=/"))
    expect("a Domain cookie replaces a host-only cookie of the same name", replaced.headerFor(origin), "sid=b")
    replaced.absorb(origin, listOf("sid=b; Path=/; Max-Age=0"))
    expect("Max-Age=0 without Domain deletes the Domain cookie", replaced.headerFor(origin), null)
    expect(
        "Max-Age=0 without Domain deletes the Domain cookie for a subdomain too",
        replaced.headerFor(URI("https://a.example.com/")),
        null,
    )

    handler()

    if (failures.isEmpty()) {
        println("HARNESS OK")
    } else {
        failures.forEach { println("FAILED: $it") }
        exitProcess(1)
    }
}

/** One request as the server saw it. */
class Seen(
    val method: String,
    val path: String,
    val body: String,
    val cookie: String?,
    val authorization: String?,
    val contentType: String?,
)

/** The request a test sends: a method, a URL, headers and a body. */
fun request(
    method: String,
    url: String,
    headers: List<HttpHeader> = emptyList(),
    body: String = "",
) = HttpRequest(method, url, headers, Bytes(body.toByteArray()))

/** The status of an `Ok`, or a description of what came back instead. */
fun status(result: HttpResult): String =
    when (result) {
        is HttpResult.Ok -> "Ok(${result.value.status})"
        is HttpResult.Err -> "Err(${result.value})"
    }

/** Whether [result] is an `Err` of the kind that [kind] accepts. */
fun failed(
    result: HttpResult,
    kind: (HttpError) -> Boolean,
): Boolean = result is HttpResult.Err && kind(result.value)

fun handler() {
    val seen = java.util.Collections.synchronizedList(mutableListOf<Seen>())
    val server = HttpServer.create(InetSocketAddress(InetAddress.getLoopbackAddress(), 0), 0)
    val port = server.address.port

    fun respond(
        exchange: HttpExchange,
        status: Int,
        vararg headers: Pair<String, String>,
    ) {
        for ((name, value) in headers) exchange.responseHeaders.add(name, value)
        exchange.sendResponseHeaders(status, -1)
        exchange.close()
    }

    server.createContext("/") { exchange ->
        val request = exchange.requestHeaders
        seen.add(
            Seen(
                exchange.requestMethod,
                exchange.requestURI.path,
                String(exchange.requestBody.readBytes()),
                request.getFirst("Cookie"),
                request.getFirst("Authorization"),
                request.getFirst("Content-Type"),
            ),
        )
        when (exchange.requestURI.path) {
            "/found" -> respond(exchange, 302, "Location" to "/echo")
            "/found-with-cookie" -> respond(exchange, 302, "Location" to "/echo", "Set-Cookie" to "hop=1; Path=/")
            "/temporary" -> respond(exchange, 307, "Location" to "/echo")
            "/elsewhere" -> respond(exchange, 302, "Location" to "http://127.0.0.1:$port/echo")
            "/loop" -> respond(exchange, 302, "Location" to "/loop")
            else -> respond(exchange, 200)
        }
    }
    server.start()

    // A hung answer would otherwise wait for the test's own timeout, with
    // nothing to say for itself.
    Thread {
        Thread.sleep(60_000)
        System.err.println("timed out: a request never answered")
        exitProcess(2)
    }.apply { isDaemon = true }.start()

    val http =
        UrlConnectionHttpHandler(configure = { connection ->
            connection.connectTimeout = 5_000
            connection.readTimeout = 5_000
        })
    val local = "http://localhost:$port"

    // Each request is run on its own, and an exception is a failure rather
    // than the end of the run, so that one run reports every case.
    fun send(
        what: String,
        operation: HttpRequest,
    ): HttpResult? =
        try {
            runBlocking { http.request(operation) }
        } catch (e: Exception) {
            failures.add("$what: threw $e")
            null
        }

    fun last(): Seen? = seen.lastOrNull()

    send("a file URL", request("GET", "file:///etc/hosts"))?.let {
        expect("a file URL is HttpError.Url, got ${status(it)}", failed(it) { e -> e is HttpError.Url })
    }

    send("a header value with CR LF", request("GET", "$local/echo", listOf(HttpHeader("X-Test", "a\r\nInjected: yes"))))?.let {
        expect("a header value with CR LF is HttpError.Io, got ${status(it)}", failed(it) { e -> e is HttpError.Io })
    }

    // `HttpURLConnection` sends a GET with a body as a POST.
    val before = seen.size
    send("a GET with a body", request("GET", "$local/echo", body = "body"))?.let {
        expect("a GET with a body is HttpError.Io, got ${status(it)}", failed(it) { e -> e is HttpError.Io })
    }
    expect("a GET with a body does not reach the server", seen.size, before)

    // A 302 after a POST is a GET without the body or the headers that
    // describe it.
    val post = listOf(HttpHeader("Content-Type", "text/plain"))
    send("a POST that a 302 redirects", request("POST", "$local/found", post, "form"))?.let {
        expect("a POST that a 302 redirects", status(it), "Ok(200)")
    }
    expect("a 302 after a POST is a GET", last()?.method, "GET")
    expect("a 302 after a POST goes to the Location", last()?.path, "/echo")
    expect("a 302 after a POST sends no body", last()?.body, "")
    expect("a 302 after a POST sends no Content-Type", last()?.contentType, null)

    // The jar takes the cookies of each hop, so the next hop sends them.
    send("a 302 that sets a cookie", request("GET", "$local/found-with-cookie"))?.let {
        expect("a 302 that sets a cookie", status(it), "Ok(200)")
    }
    expect("a cookie set on a 302 is sent on the next hop", last()?.cookie, "hop=1")

    send("a POST that a 307 redirects", request("POST", "$local/temporary", post, "keep"))?.let {
        expect("a POST that a 307 redirects", status(it), "Ok(200)")
    }
    expect("a 307 keeps the method", last()?.method, "POST")
    expect("a 307 keeps the body", last()?.body, "keep")

    // `localhost` and `127.0.0.1` are the same server but not the same origin.
    val credentials = listOf(HttpHeader("Authorization", "Bearer secret"), HttpHeader("Cookie", "own=1"))
    send("a redirect to another host", request("GET", "$local/elsewhere", credentials))?.let {
        expect("a redirect to another host", status(it), "Ok(200)")
    }
    val hops = seen.takeLast(2)
    expect("the first hop gets the Authorization header", hops.firstOrNull()?.authorization, "Bearer secret")
    expect("a redirect to another host goes there", hops.lastOrNull()?.path, "/echo")
    expect("a redirect to another host sends no Authorization", hops.lastOrNull()?.authorization, null)
    // The jar holds `hop=1` for `localhost` only, so no cookie is sent.
    expect("a redirect to another host sends no Cookie", hops.lastOrNull()?.cookie, null)

    send("a redirect loop", request("GET", "$local/loop"))?.let {
        expect("a redirect loop", status(it), "Err(${HttpError.Io("too many redirects")})")
    }

    // A redirect from `https` to `http` is not tested here. The first hop
    // needs a TLS server with a certificate that the JDK trusts, and the
    // harness has neither.

    server.stop(0)
}
