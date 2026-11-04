import android.webkit.CookieManager
import java.net.URI

/**
 * A [CookieJar] over Android's `CookieManager`, the cookie store that
 * `WebView` also uses.
 *
 * Use it on Android in place of [InMemoryCookieJar] to keep cookies after the
 * app stops, and to share them with any `WebView` in the app:
 *
 * ```kotlin
 * private val http = UrlConnectionHttpHandler(cookies = WebkitCookieJar())
 * ```
 *
 * `CookieManager` matches each cookie to a URL by the rules of RFC 6265, and
 * writes the `Cookie` header in the RFC 6265 form that servers read.
 *
 * ## When `WebView` is not available
 *
 * `CookieManager.getInstance()` throws if the device has no `WebView`
 * package, or while the system updates that package. Then the jar keeps
 * cookies in an [InMemoryCookieJar] instead, for as long as the jar lives.
 * [isPersistent] tells you which store the jar uses.
 *
 * ## When `CookieManager` writes to disk
 *
 * `CookieManager` keeps new cookies in memory and writes them to disk at a
 * later time. If the process stops before that write, it loses those cookies.
 * Call [flush] when the app goes to the background, for example in an
 * activity's `onStop`, to write them to disk immediately. `flush` does
 * nothing for the in-memory store.
 */
class WebkitCookieJar private constructor(
    private val cookieManager: CookieManager?,
) : CookieJar {
    /** A jar over `CookieManager.getInstance()`, or in memory if that throws. */
    constructor() : this(defaultCookieManager())

    private val fallback: CookieJar? = if (cookieManager == null) InMemoryCookieJar() else null

    /** `true` if the jar keeps cookies in `CookieManager`, `false` if in memory. */
    val isPersistent: Boolean
        get() = cookieManager != null

    override fun headerFor(url: URI): String? {
        if (cookieManager == null) {
            return fallback?.headerFor(url)
        }
        return cookieManager.getCookie(url.toString())?.takeIf { it.isNotEmpty() }
    }

    override fun absorb(
        url: URI,
        setCookie: List<String>,
    ) {
        if (cookieManager == null) {
            fallback?.absorb(url, setCookie)
            return
        }
        for (value in setCookie) {
            cookieManager.setCookie(url.toString(), value)
        }
    }

    /** Write the cookies `CookieManager` holds in memory to disk. */
    fun flush() {
        cookieManager?.flush()
    }

    private companion object {
        fun defaultCookieManager(): CookieManager? =
            try {
                CookieManager.getInstance()
            } catch (e: RuntimeException) {
                null
            }
    }
}
