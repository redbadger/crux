use anyhow::Result;
use futures_util::{stream, StreamExt};
use gloo_net::http;
use js_sys::Uint8Array;
use wasm_bindgen::{prelude::*, JsValue};
use wasm_streams::ReadableStream;

use shared::sse::{SseRequest, SseResponse};

pub async fn request(
    SseRequest { url }: &SseRequest,
) -> Result<impl stream::TryStream<Ok = SseResponse, Error = JsValue>> {
    let response = http::Request::get(url).send().await?;

    let raw_body = response.body().unwrap_throw();
    let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

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
