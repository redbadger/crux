use futures::{stream, AsyncReadExt};

use shared::sse::{SseRequest, SseResponse};

use crate::error::Error;

pub async fn request(
    SseRequest { url }: &SseRequest,
) -> Result<impl futures::TryStream<Ok = SseResponse>, Error> {
    let mut response = surf::get(url).await?;
    let status = response.status().into();

    let body = if let 200..=299 = status {
        response.take_body()
    } else {
        return Err(Error::HttpResponse(status, "SSE error".to_string()));
    };

    let body = body.into_reader();

    Ok(Box::pin(stream::try_unfold(body, |mut body| async {
        let mut buf = [0; 1024];

        match body.read(&mut buf).await {
            Ok(0) => Ok(None),
            Ok(n) => {
                let chunk = SseResponse::Chunk(buf[0..n].to_vec());
                Ok(Some((chunk, body)))
            }
            Err(e) => Err(Error::HttpDecode(format!(
                "failed to read from http response; err = {:?}",
                e
            ))),
        }
    })))
}
