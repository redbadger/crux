//! The `ServerSentEvents` effect, as in counter-http's Leptos shell.

use futures_util::{StreamExt, stream};
use gloo_net::http;
use js_sys::Uint8Array;
use wasm_bindgen::{JsValue, prelude::*};
use wasm_streams::ReadableStream;

use cpb_protocol::{SseRequest, SseResponse};

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn request(
    SseRequest { url }: &SseRequest,
) -> Result<impl stream::Stream<Item = Result<SseResponse, JsValue>>, String> {
    let response = http::Request::get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let raw_body = response.body().ok_or("response has no body")?;
    let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

    Ok(body
        .into_stream()
        .map(|chunk| chunk.map(|chunk| SseResponse::Chunk(Uint8Array::from(chunk).to_vec()))))
}
