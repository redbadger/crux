use std::rc::Rc;

use anyhow::Result;
use futures::{stream, TryStreamExt};
use gloo_net::http;
use wasm_bindgen::JsValue;
use yew::{html::Scope, prelude::*};

use shared::{
    http::protocol::{HttpHeader, HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    App, Capabilities, Core, Effect, Event,
};

#[derive(Default)]
struct RootComponent {
    core: Rc<Core<Effect, App>>,
}

enum Task {
    Event(Event),
    Effect(Effect),
}

fn send_effects(link: &Scope<RootComponent>, effects: Vec<Effect>) {
    link.send_message_batch(effects.into_iter().map(Task::Effect).collect());
}

impl Component for RootComponent {
    type Message = Task;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(Task::Event(Event::StartWatch));

        Self {
            core: Rc::new(Core::new::<Capabilities>()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();
        let core = &self.core;

        match msg {
            Task::Event(event) => send_effects(link, core.process_event(event)),
            Task::Effect(effect) => match effect {
                Effect::Render(_) => return true,
                Effect::Http(mut request) => {
                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();
                        let core = core.clone();

                        let HttpRequest {
                            url,
                            method,
                            headers,
                            body: _,
                        } = request.operation.clone();

                        let method = match method.as_str() {
                            "GET" => http::Method::GET,
                            "POST" => http::Method::POST,
                            _ => panic!("not yet handling this method"),
                        };

                        async move {
                            let response = http(&url, method, &headers).await.unwrap();

                            send_effects(&link, core.resolve(&mut request, response));
                        }
                    });
                }
                Effect::ServerSentEvents(mut request) => {
                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();
                        let core = core.clone();

                        let SseRequest { url } = request.operation.clone();

                        async move {
                            let mut stream = sse(&url).await.unwrap();

                            while let Ok(Some(chunk)) = stream.try_next().await {
                                let response = SseResponse::Chunk(chunk);
                                send_effects(&link, core.resolve(&mut request, response));
                            }
                        }
                    });
                }
            },
        };

        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = self.core.view();

        html! {
            <>
                <section class="section has-text-centered">
                    <p class="title">{"Crux Counter Example"}</p>
                </section>
                <section class="section has-text-centered">
                    <p class="is-size-5">{"Rust Core, Rust Shell (Yew)"}</p>
                </section>
                <section class="container has-text-centered">
                    <p class="is-size-5">{&view.text}</p>
                    <div class="buttons section is-centered">
                        <button class="button is-primary is-warning"
                            onclick={link.callback(|_| Task::Event(Event::Decrement))}>
                            {"Decrement"}
                        </button>
                        <button class="button is-primary is-danger"
                            onclick={link.callback(|_| Task::Event(Event::Increment))}>
                            {"Increment"}
                        </button>
                    </div>
                </section>
            </>
        }
    }
}

async fn http(url: &str, method: http::Method, headers: &[HttpHeader]) -> Result<HttpResponse> {
    let mut request = http::Request::new(url).method(method);

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

async fn sse(url: &str) -> Result<impl futures::stream::TryStream<Ok = Vec<u8>, Error = JsValue>> {
    use futures_util::StreamExt;
    use js_sys::Uint8Array;
    use wasm_bindgen::{prelude::*, JsCast};
    use wasm_streams::ReadableStream;

    let response = http::Request::new(url).send().await?;

    let raw_body = response.body().unwrap_throw();
    let body = ReadableStream::from_raw(raw_body.dyn_into().unwrap_throw());

    let stream = body.into_stream();

    Ok(Box::pin(stream::try_unfold(stream, |mut stream| async {
        match stream.next().await {
            Some(Ok(chunk)) => {
                let chunk: Uint8Array = chunk
                    .try_into()
                    .expect("should cast from JSValue into bytes");

                Ok(Some((chunk.to_vec(), stream)))
            }
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    })))
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
