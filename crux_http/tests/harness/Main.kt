import com.example.shared.InMemoryCookieJar
import java.net.URI
import kotlin.system.exitProcess

/// Runs `InMemoryCookieJar` through RFC 6265's matching rules and prints
/// `HARNESS OK` when every answer was the one a server would expect.
///
/// The jar, not the handler: what it puts in a `Cookie` header is all
/// behaviour, and none of it is a compile error. The case that matters most is
/// the first — a cookie carrying `Max-Age`, which `java.net.CookieManager`
/// would write back in RFC 2965 form (`$Version="1"; sid="abc";$Path="/"`) that
/// no server reads. That is why this jar exists, so that is what is asserted.

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

    if (failures.isEmpty()) {
        println("HARNESS OK")
    } else {
        failures.forEach { println("FAILED: $it") }
        exitProcess(1)
    }
}
