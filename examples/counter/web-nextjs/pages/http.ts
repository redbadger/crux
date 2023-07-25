import {
  HttpRequest,
  HttpResponse,
  HttpHeader,
} from "shared_types/types/shared_types";

export async function http(httpRequest: HttpRequest): Promise<HttpResponse> {
  const request = new Request(httpRequest.url, {
    method: httpRequest.method,
    headers: httpRequest.headers.map((header) => [header.name, header.value]),
  });

  const response = await fetch(request);

  const responseHeaders = Array.from(
    response.headers.entries(),
    ([name, value]) => new HttpHeader(name, value)
  );

  const body = await response.arrayBuffer();

  const httpResponse = new HttpResponse(
    response.status,
    responseHeaders,
    Array.from(new Uint8Array(body))
  );
  return httpResponse;
}
