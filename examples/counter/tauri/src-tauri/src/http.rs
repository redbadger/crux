use crate::error::Error;
use shared::http::protocol::{HttpRequest, HttpResponse};
use std::str::FromStr;
use std::time::Duration;

pub async fn http(http_request: &HttpRequest) -> Result<HttpResponse, Error> {
    let HttpRequest {
        ref method,
        ref url,
        ref headers,
        body: _,
    } = http_request;

    let method = surf::http::Method::from_str(method).expect("unknown http method");
    let url = surf::Url::parse(url)?;
    let headers = headers.clone();

    let client: surf::Client = surf::Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()
        .map_err(|_e| Error::HttpConfig("Http client config error".to_string()))?;

    let mut request = client.request(method, &url);

    for header in headers {
        request = request.header(header.name.as_str(), &header.value);
    }

    let mut response = request.await?;
    let status = response.status().into();

    match response.body_bytes().await {
        Ok(body) => Ok(HttpResponse { status, body }),
        Err(e) => Err(e.into()),
    }
}
