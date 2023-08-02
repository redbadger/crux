use std::{str::FromStr, time::Duration};

use surf::{http::Method, Client, Config, Url};

use shared::http::protocol::{HttpRequest, HttpResponse};

use crate::error::Error;

pub async fn request(
    HttpRequest {
        method,
        url,
        headers,
        ..
    }: &HttpRequest,
) -> Result<HttpResponse, Error> {
    let method = Method::from_str(method).expect("unknown http method");
    let url = Url::parse(url)?;
    let headers = headers.clone();

    let client: Client = Config::new()
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()
        .map_err(|_e| Error::HttpConfig("Http client config error".to_string()))?;

    let mut request = client.request(method, &url);

    for header in headers {
        request = request.header(header.name.as_str(), &header.value);
    }

    let mut response = request.await?;
    let status = response.status().into();

    let body = response.body_bytes().await?;
    Ok(HttpResponse::status(status).body(body).build())
}
