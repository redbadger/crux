/// The shell side of `crux_http`, as one method per operation.
///
/// The method's shape is the generated `EffectHandler`'s, so an app's handler
/// holds an instance and delegates in one line:
///
/// ```ts
/// const handler: EffectHandler = {
///   http: (operation) => fetchHttpHandler.request(operation),
/// };
/// ```
///
/// Write an object of your own shape to replace the implementation wholesale.
export interface HttpHandler {
  /// Perform `operation` and answer with its result, exactly once.
  request(operation: HttpRequest): Promise<HttpResult>;
}

/// An `HttpHandler` over `fetch`.
///
/// ## What is a failure
///
/// Every exchange `fetch` completes is an `HttpResult` of kind `Ok`, whatever
/// the status: 4xx and 5xx are answers, and the core decides what to make of
/// them. Only a transport failure — the request never reached a server, or no
/// response came back — is an `Err`, and then a URL that will not parse is
/// `httpErrorUrl`, an abort or timeout is `httpErrorTimeout`, and everything
/// else is `httpErrorIo`.
///
/// @param fetchImpl the `fetch` to use; the global one when omitted, read at
///   call time so that a test can install its own first.
export function createFetchHttpHandler(
  fetchImpl?: typeof globalThis.fetch,
): HttpHandler {
  return {
    async request(operation: HttpRequest): Promise<HttpResult> {
      try {
        // `globalThis`, because the generated module may well declare types of
        // its own called `URL` or `Request` — `crux_core` already contributes a
        // `Request` — and they would shadow the platform's.
        new globalThis.URL(operation.url);
      } catch {
        return httpResultErr(
          httpErrorUrl(`could not parse URL: ${operation.url}`),
        );
      }

      const method = operation.method.toUpperCase();
      const init: RequestInit = {
        method,
        // Header names and values cross the boundary as plain strings, with no
        // validation — `Headers` applies its own rules.
        headers: operation.headers.map(
          (header) => [header.name, header.value] as [string, string],
        ),
        // `fetch` rejects a GET or HEAD that carries a body, and an empty body
        // is no body.
        body:
          operation.body.length > 0 && method !== "GET" && method !== "HEAD"
            ? // Copy, because the request may outlive the operation's buffer.
              new Uint8Array(operation.body)
            : null,
      };

      try {
        const doFetch = fetchImpl ?? globalThis.fetch;
        const response = await doFetch(operation.url, init);

        const headers: HttpHeader[] = [];
        response.headers.forEach((value, name) => {
          headers.push(new HttpHeader(name, value));
        });
        const body = new Uint8Array(await response.arrayBuffer());

        return httpResultOk(new HttpResponse(response.status, headers, body));
      } catch (error) {
        const name = error instanceof Error ? error.name : "";
        if (name === "AbortError" || name === "TimeoutError") {
          return httpResultErr(httpErrorTimeout());
        }
        return httpResultErr(
          httpErrorIo(error instanceof Error ? error.message : String(error)),
        );
      }
    },
  };
}

/// The plain path: one handler over the global `fetch`.
export const fetchHttpHandler: HttpHandler = createFetchHttpHandler();
