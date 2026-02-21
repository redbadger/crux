use futures::{StreamExt, stream};

use reqwest::{Client, Method};
use shared::{
    http::{HttpError, Result},
    sse::{SseRequest, SseResponse},
};

pub async fn request(
    SseRequest { url }: &SseRequest,
) -> Result<impl futures::TryStream<Ok = SseResponse>> {
    let client = Client::new();
    let method = Method::from_bytes(b"GET").unwrap();

    let request = client
        .request(method, url)
        .build()
        .map_err(|e| HttpError::Url(e.to_string()))?;

    let response = client
        .execute(request)
        .await
        .map_err(|e| HttpError::Io(e.to_string()))?;

    let body = response.bytes_stream();

    Ok(Box::pin(stream::try_unfold(body, |mut body| async {
        body.next().await.map_or_else(
            || Ok(None),
            |chunk| match chunk {
                Ok(bytes) => {
                    let chunk = SseResponse::Chunk(bytes.to_vec());
                    Ok(Some((chunk, body)))
                }
                Err(e) => Err(HttpError::Json(format!(
                    "failed to read from http response; err = {e:?}"
                ))),
            },
        )
    })))
}
