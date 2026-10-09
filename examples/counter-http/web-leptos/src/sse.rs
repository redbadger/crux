use anyhow::{Context as _, Result, bail};
use futures_util::{StreamExt, stream};
use gloo_net::http;
use js_sys::Uint8Array;
use wasm_bindgen::{JsValue, prelude::*};
use wasm_streams::ReadableStream;

use shared::sse::{SseRequest, SseResponse};

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn request(
    SseRequest { url }: &SseRequest,
) -> Result<impl stream::TryStream<Ok = SseResponse, Error = JsValue>> {
    let response = http::Request::get(url).send().await?;

    if !response.ok() {
        bail!("SSE request failed: {}", response.status());
    }
    let raw_body = response.body().context("SSE response has no body")?;
    let body = ReadableStream::from_raw(raw_body.unchecked_into());

    let stream = body.into_stream();

    Ok(Box::pin(stream::try_unfold(stream, |mut stream| async {
        match stream.next().await {
            None => Ok(None),
            Some(Ok(chunk)) => {
                let chunk: Uint8Array = chunk.into();
                let response = SseResponse::Chunk(chunk.to_vec());

                Ok(Some((response, stream)))
            }
            Some(Err(e)) => Err(e),
        }
    })))
}
