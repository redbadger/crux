use gloo_net::http;

use shared::http::{
    protocol::{HttpRequest, HttpResponse},
    HttpError, Result,
};

pub async fn request(
    HttpRequest {
        method,
        url,
        headers,
        ..
    }: &HttpRequest,
) -> Result<HttpResponse> {
    let mut request = match method.as_str() {
        "GET" => http::Request::get(url),
        "POST" => http::Request::post(url),
        _ => panic!("not yet handling this method"),
    };

    for header in headers {
        request = request.header(&header.name, &header.value);
    }

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
