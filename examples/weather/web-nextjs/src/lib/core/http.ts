import type { HttpHandler, HttpRequest, HttpResult } from "shared_types/app";
import {
  HttpHeader,
  HttpResponse,
  httpErrorIo,
  httpErrorTimeout,
  httpErrorUrl,
  httpResultErr,
  httpResultOk,
} from "shared_types/app";

/**
 * The app's `HttpHandler`, over the browser's `fetch`.
 *
 * Every response is an answer, whatever its status. Only a transport failure
 * is an error, as the protocol says.
 */
export class FetchHttpHandler implements HttpHandler {
  async request({
    method,
    url,
    headers,
    body,
  }: HttpRequest): Promise<HttpResult> {
    console.debug(`http ${method} ${url}`);

    // Without a base, `new URL` accepts only an absolute URL.
    try {
      new URL(url);
    } catch {
      return httpResultErr(httpErrorUrl(`not an absolute URL: ${url}`));
    }

    const hasBody = body.length > 0;
    if (hasBody && ["GET", "HEAD"].includes(method.toUpperCase())) {
      return httpResultErr(
        httpErrorIo(`a ${method} request cannot have a body`),
      );
    }

    try {
      const response = await fetch(url, {
        method,
        headers: headers.map((header): [string, string] => [
          header.name,
          header.value,
        ]),
        body: hasBody ? new Uint8Array(body) : undefined,
      });

      const responseHeaders = Array.from(
        response.headers.entries(),
        ([name, value]) => new HttpHeader(name, value),
      );
      const responseBody = await response.arrayBuffer();

      return httpResultOk(
        new HttpResponse(
          response.status,
          responseHeaders,
          new Uint8Array(responseBody),
        ),
      );
    } catch (error) {
      if (
        error instanceof DOMException &&
        (error.name === "AbortError" || error.name === "TimeoutError")
      ) {
        return httpResultErr(httpErrorTimeout());
      }
      return httpResultErr(httpErrorIo(String(error)));
    }
  }
}
