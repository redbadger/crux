use gloo_net::http;
use leptos::prelude::*;

use shared::ViewModel;
use shared::http::{
    HttpError,
    protocol::{HttpRequest, HttpResponse},
};

// ANCHOR: resolve
pub(super) fn resolve(
    core: &super::Core,
    mut request: crux_core::Request<HttpRequest>,
    render: WriteSignal<ViewModel>,
) {
    let core = core.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let op = &request.operation;
        log::debug!("http {} {}", op.method, op.url);

        let response = send_request(op).await;
        super::resolve_effect(&core, &mut request, response.into(), render);
    });
}
// ANCHOR_END: resolve

async fn send_request(
    HttpRequest {
        method,
        url,
        headers,
        ..
    }: &HttpRequest,
) -> shared::http::Result<HttpResponse> {
    let mut request = match method.as_str() {
        "GET" => http::Request::get(url),
        "POST" => http::Request::post(url),
        "PUT" => http::Request::put(url),
        "DELETE" => http::Request::delete(url),
        "PATCH" => http::Request::patch(url),
        _ => return Err(HttpError::Io(format!("unsupported HTTP method: {method}"))),
    };

    for header in headers {
        request = request.header(&header.name, &header.value);
    }

    let response = request
        .send()
        .await
        .map_err(|e| HttpError::Io(e.to_string()))?;

    let body = response
        .binary()
        .await
        .map_err(|e| HttpError::Io(e.to_string()))?;

    Ok(HttpResponse::status(response.status()).body(body).build())
}
