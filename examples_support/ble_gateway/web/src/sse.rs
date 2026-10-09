//! The `SseStream` effect: as in counter-http's Leptos shell, but telling the core whether
//! the stream ended or failed.

use futures_util::{StreamExt, stream};
use gloo_net::http;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_streams::ReadableStream;

use gateway_core::SseStream;

/// Open the stream: its body's chunks, or why each read failed. `Err` if it never opened
/// (the fetch was rejected, or the server answered with something other than 2xx).
#[allow(clippy::future_not_send)] // WASM is single-threaded
pub async fn request(
    SseStream { url }: &SseStream,
) -> Result<impl stream::Stream<Item = Result<Vec<u8>, String>> + use<>, String> {
    let response = http::Request::get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.ok() {
        return Err(format!("{} {}", response.status(), response.status_text()));
    }

    let raw_body = response.body().ok_or("response has no body")?;
    let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

    Ok(body.into_stream().map(|chunk| {
        chunk
            .map(|chunk| Uint8Array::from(chunk).to_vec())
            .map_err(|error| format!("{error:?}"))
    }))
}
