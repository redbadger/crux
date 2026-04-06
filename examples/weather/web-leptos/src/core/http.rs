use gloo_net::http;
use leptos::prelude::*;

use shared::ViewModel;
use shared::http::{
    HttpError,
    protocol::{HttpRequest, HttpResponse},
};

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

        match core.resolve(&mut request, response.into()) {
            Ok(new_effects) => super::process_effects(&core, new_effects, render),
            Err(e) => log::warn!("failed to resolve http: {e:?}"),
        }
    });
}

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
        _ => panic!("unsupported HTTP method: {method}"),
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
