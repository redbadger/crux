use anyhow::Result;
use shared::http::protocol::{HttpRequest, HttpResponse};

pub async fn request(request: &HttpRequest) -> Result<HttpResponse> {
    use gloo_net::http;

    let HttpRequest {
        url,
        method,
        headers,
        body: _,
    } = request;

    let mut request = match method.as_str() {
        "GET" => http::Request::get(url),
        "POST" => http::Request::post(url),
        _ => panic!("not yet handling this method"),
    };

    for header in headers {
        request = request.header(&header.name, &header.value);
    }

    let response = request.send().await?;
    let body = response.binary().await?;

    Ok(HttpResponse {
        status: response.status(),
        body,
        ..Default::default()
    })
}
