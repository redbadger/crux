use anyhow::Result;
use bcs::{from_bytes, to_bytes};
use gloo_net::http;
use js_sys::{Object, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::ReadableStreamDefaultReader;
use yew::{html::Scope, prelude::*};

use shared::{
    http::protocol::{HttpRequest, HttpResponse},
    sse::{SseRequest, SseResponse},
    Effect, Event, Request, ViewModel,
};

#[derive(Default)]
struct RootComponent;

enum CoreMessage {
    Message(Event),
    Response(Vec<u8>, Outcome),
}

pub enum Outcome {
    Http(HttpResponse),
    Sse(Option<SseResponse>),
}

impl Component for RootComponent {
    type Message = CoreMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(CoreMessage::Message(Event::Get));
        link.send_message(CoreMessage::Message(Event::GetServerEvents));

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        let reqs = match msg {
            CoreMessage::Message(event) => shared::message(&to_bytes(&event).unwrap()),
            CoreMessage::Response(uuid, outcome) => shared::response(
                &uuid,
                &match outcome {
                    Outcome::Http(x) => to_bytes(&x).unwrap(),
                    Outcome::Sse(x) => to_bytes(&x).unwrap(),
                },
            ),
        };

        let reqs: Vec<Request<Effect>> = from_bytes(&reqs).unwrap();

        let mut should_render = false;

        for Request { uuid, effect } in reqs {
            match effect {
                Effect::Render(_) => should_render = true,
                Effect::Http(HttpRequest { url, method }) => {
                    let method = match method.as_str() {
                        "GET" => http::Method::GET,
                        "POST" => http::Method::POST,
                        _ => panic!("not yet handling this method"),
                    };

                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();

                        async move {
                            if let Ok(body) = http(&url, method).await {
                                link.send_message(CoreMessage::Response(
                                    uuid,
                                    Outcome::Http(HttpResponse { status: 200, body }), // TODO: handle status
                                ));
                            }
                        }
                    });
                }
                Effect::ServerSentEvents(SseRequest { url }) => {
                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();

                        async move {
                            sse(&uuid, &url, &link).await.unwrap();
                        }
                    });
                }
            }
        }

        should_render
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = shared::view();
        let view: ViewModel = from_bytes(&view).unwrap();

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
                            onclick={link.callback(|_| CoreMessage::Message(Event::Decrement))}>
                            {"Decrement"}
                        </button>
                        <button class="button is-primary is-danger"
                            onclick={link.callback(|_| CoreMessage::Message(Event::Increment))}>
                            {"Increment"}
                        </button>
                    </div>
                </section>
            </>
        }
    }
}

async fn http(url: &str, method: http::Method) -> Result<Vec<u8>> {
    let bytes = http::Request::new(url)
        .method(method)
        .send()
        .await?
        .binary()
        .await?;
    Ok(bytes)
}

async fn sse(uuid: &[u8], url: &str, link: &Scope<RootComponent>) -> Result<()> {
    if let Some(body) = http::Request::new(url)
        .method(http::Method::GET)
        .send()
        .await?
        .body()
    {
        let reader = body.get_reader();
        let reader: ReadableStreamDefaultReader = reader.dyn_into().unwrap();
        loop {
            let result = JsFuture::from(reader.read()).await.unwrap();
            let result: Object = result.dyn_into().unwrap();
            let chunk = js_sys::Reflect::get(&result, &JsValue::from_str("value")).unwrap();
            let chunk: Uint8Array = chunk.dyn_into().unwrap();
            link.send_message(CoreMessage::Response(
                uuid.to_vec(),
                Outcome::Sse(Some(SseResponse::Raw(chunk.to_vec()))),
            ));
        }
    }
    Ok(())
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
