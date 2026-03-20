import type { HttpRequest, HttpResult } from "app/app";
import { HttpResponse, HttpHeader, HttpResultVariantOk } from "app/app";

export async function request({
  url,
  method,
  headers,
}: HttpRequest): Promise<HttpResult> {
  const request = new Request(url, {
    method,
    headers: headers.map((header) => [header.name, header.value]),
  });

  const response = await fetch(request);

  const responseHeaders = Array.from(
    response.headers.entries(),
    ([name, value]) => new HttpHeader(name, value)
  );

  const body = await response.arrayBuffer();

  return new HttpResultVariantOk(
    new HttpResponse(
      response.status,
      responseHeaders,
      Array.from(new Uint8Array(body))
    )
  );
}
