use std::rc::Rc;

use anyhow::Result;
use futures_util::{stream, TryStreamExt};
use leptos::{
    component, create_effect, create_signal, spawn_local, view, IntoView, Scope, SignalGet,
    SignalUpdate, WriteSignal,
};
use serde::{Deserialize, Serialize};
use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    App, Capabilities, Core, Effect, Event, ViewModel,
};
use wasm_bindgen::JsValue;

// we need a newtype for Event because signals require Clone, so use json instead
#[derive(Debug, Serialize, Deserialize)]
struct Task(Event);

impl Task {
    fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Into::into)
    }
}

#[component]
fn RootComponent(cx: Scope) -> impl IntoView {
    let core = Rc::new(Core::new::<Capabilities>());
    let (view_model, set_view_model) = create_signal(cx, core.view());
    let (event, set_event) = create_signal(cx, Task(Event::StartWatch).to_json());

    create_effect(cx, move |_| {
        let event = event.get();
        log::debug!("event: {:?}", event);
        if let Ok(Task(event)) = Task::from_json(&event) {
            for effect in core.process_event(event) {
                process_effect(effect, &core, set_view_model);
            }
        }
    });

    view! {cx,
        <>
            <section class="section has-text-centered">
                <p class="title">{"Crux Counter Example"}</p>
            </section>
            <section class="section has-text-centered">
                <p class="is-size-5">{"Rust Core, Rust Shell (Leptos)"}</p>
            </section>
            <section class="container has-text-centered">
                <p class="is-size-5">{move || view_model.get().text}</p>
                <div class="buttons section is-centered">
                    <button class="button is-primary is-warning"
                        on:click=move |_| set_event.update(|value| *value = Task(Event::Decrement).to_json())
                    >
                        {"Decrement"}
                    </button>
                    <button class="button is-primary is-danger"
                    on:click=move |_| set_event.update(|value| *value = Task(Event::Increment).to_json())
                    >
                        {"Increment"}
                    </button>
                </div>
            </section>
        </>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount_to_body(|cx| {
        view! { cx, <RootComponent /> }
    });
}

fn process_effect(effect: Effect, core: &Rc<Core<Effect, App>>, render: WriteSignal<ViewModel>) {
    log::debug!("effect: {:?}", effect);

    match effect {
        Effect::Render(_) => {
            render.update(|view_model| *view_model = core.view());
        }
        Effect::Http(mut request) => {
            spawn_local({
                let core = core.clone();

                async move {
                    let response = http(&request.operation).await.unwrap();
                    for effect in core.resolve(&mut request, response) {
                        process_effect(effect, &core, render);
                    }
                }
            });
        }
        Effect::ServerSentEvents(mut request) => {
            spawn_local({
                let core = core.clone();

                async move {
                    let mut stream = sse(&request.operation).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response) {
                            process_effect(effect, &core, render);
                        }
                    }
                }
            });
        }
    };
}

async fn http(request: &HttpRequest) -> Result<HttpResponse> {
    use gloo_net::http;

    let HttpRequest {
        url,
        method,
        headers,
        body: _,
    } = request;

    let mut request = match method.as_str() {
        "GET" => http::Request::get(url),
        "POST" => http::Request::post(url),
        _ => panic!("not yet handling this method"),
    };

    for header in headers {
        request = request.header(&header.name, &header.value);
    }

    let response = request.send().await?;
    let body = response.binary().await?;

    Ok(HttpResponse {
        status: response.status(),
        body,
    })
}

async fn sse(
    request: &SseRequest,
) -> Result<impl futures::stream::TryStream<Ok = SseResponse, Error = JsValue>> {
    use futures_util::StreamExt;
    use gloo_net::http;
    use js_sys::Uint8Array;
    use wasm_bindgen::prelude::*;
    use wasm_streams::ReadableStream;

    let response = http::Request::get(&request.url).send().await?;

    let raw_body = response.body().unwrap_throw();
    let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

    let stream = body.into_stream();

    Ok(Box::pin(stream::try_unfold(stream, |mut stream| async {
        match stream.next().await {
            Some(Ok(chunk)) => {
                let chunk: Uint8Array = chunk.into();
                let response = SseResponse::Chunk(chunk.to_vec());

                Ok(Some((response, stream)))
            }
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    })))
}
