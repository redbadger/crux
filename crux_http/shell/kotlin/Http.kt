/**
 * The shell side of `crux_http`, as one method per operation.
 *
 * The shape of the method is the shape of the matching method of the
 * generated `EffectHandler`. Thus an app's handler can hold an `HttpHandler`
 * and delegate the operation to it in one line.
 *
 * ## What is a failure
 *
 * Every exchange that completes is an `HttpResult.Ok`, whatever the status. A
 * 4xx or 5xx status is an answer, and the core decides what to do with it.
 * Only a transport failure is an `HttpResult.Err`. That is, the request did
 * not reach a server, or no response came back. Then:
 *
 * - a URL that does not parse is `HttpError.Url`
 * - a request that times out is `HttpError.Timeout`
 * - every other transport failure is `HttpError.Io`, with a description of
 *   the error
 *
 * The core builds `operation.url` with `url::Url`, so the URL is absolute. A
 * relative URL is `HttpError.Url`. A `GET` or `HEAD` request with a body is
 * `HttpError.Io`, and the handler does not send it.
 */
interface HttpHandler {
    /** Perform `operation` and answer with its result, exactly once. */
    suspend fun request(operation: HttpRequest): HttpResult
}
