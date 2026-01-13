use reqwest::{Client, Method};
use shared::http::{
    HttpError, Result,
    protocol::{HttpHeader, HttpRequest, HttpResponse},
};

pub async fn request(
    HttpRequest {
        method,
        url,
        headers,
        body,
    }: &HttpRequest,
) -> Result<HttpResponse> {
    let client = Client::new();
    let method =
        Method::from_bytes(method.as_bytes()).map_err(|e| HttpError::Url(e.to_string()))?;

    let headers = headers.iter().map(|header| {
        let name = reqwest::header::HeaderName::from_bytes(header.name.as_bytes())
            .expect("Invalid header name");
        let value = reqwest::header::HeaderValue::from_bytes(header.value.as_bytes())
            .expect("Invalid header value");

        (name, value)
    });

    let request = client
        .request(method, url)
        .headers(headers.collect::<reqwest::header::HeaderMap<_>>())
        .body(body.clone())
        .build()
        .map_err(|e| HttpError::Url(e.to_string()))?;

    let response = client
        .execute(request)
        .await
        .map_err(|e| HttpError::Io(e.to_string()))?;

    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            value
                .to_str()
                .map(|v| HttpHeader {
                    name: name.to_string(),
                    value: v.to_string(),
                })
                .map_err(|e| HttpError::Io(e.to_string()))
        })
        .collect::<Result<Vec<HttpHeader>>>()?;

    Ok(HttpResponse {
        status: response.status().as_u16(),
        headers,
        body: response
            .bytes()
            .await
            .map_err(|e| HttpError::Io(e.to_string()))?
            .to_vec(),
    })
}
