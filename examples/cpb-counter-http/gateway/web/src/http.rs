//! The `Http` effect, as in counter-http's Leptos shell, plus every method and the
//! request body (the device's POSTs may carry one).

use gloo_net::http;

use crux_http::{
    HttpError, Result,
    protocol::{HttpHeader, HttpRequest, HttpResponse},
};

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn request(
    HttpRequest {
        method,
        url,
        headers,
        body,
    }: &HttpRequest,
) -> Result<HttpResponse> {
    let method = match method.as_str() {
        "GET" => http::Method::GET,
        "HEAD" => http::Method::HEAD,
        "POST" => http::Method::POST,
        "PUT" => http::Method::PUT,
        "DELETE" => http::Method::DELETE,
        "PATCH" => http::Method::PATCH,
        "OPTIONS" => http::Method::OPTIONS,
        other => return Err(HttpError::Io(format!("unsupported method {other}"))),
    };

    let mut request = http::RequestBuilder::new(url).method(method);
    for HttpHeader { name, value } in headers {
        request = request.header(name, value);
    }
    let request = if body.is_empty() {
        request.build()
    } else {
        request.body(js_sys::Uint8Array::from(body.as_slice()))
    }
    .map_err(|error| HttpError::Io(error.to_string()))?;

    let response = request
        .send()
        .await
        .map_err(|error| HttpError::Io(error.to_string()))?;
    let body = response
        .binary()
        .await
        .map_err(|error| HttpError::Io(error.to_string()))?;

    Ok(HttpResponse::status(response.status()).body(body).build())
}
