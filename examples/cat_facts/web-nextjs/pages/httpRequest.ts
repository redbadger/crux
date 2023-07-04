import {
  HttpRequest,
  HttpResponse,
  HttpHeader,
} from "shared_types/types/shared_types";

export async function httpRequest(
  httpRequest: HttpRequest
): Promise<HttpResponse> {
  const req = new Request(httpRequest.url, {
    method: httpRequest.method,
    headers: httpRequest.headers.map((header) => [header.name, header.value]),
  });

  const res = await fetch(req);

  const responseHeaders = Array.from(
    res.headers.entries(),
    ([name, value]) => new HttpHeader(name, value)
  );

  const body = await res.arrayBuffer();

  const httpResponse = new HttpResponse(
    res.status,
    responseHeaders,
    Array.from(new Uint8Array(body))
  );
  return httpResponse;
}
