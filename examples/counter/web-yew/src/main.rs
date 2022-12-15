use anyhow::Result;
use gloo_net::http;
use yew::prelude::*;

use shared::{
    http::{HttpRequest, HttpResponse},
    Effect, Event, Request, ViewModel,
};

async fn http(url: &str, method: http::Method) -> Result<Vec<u8>> {
    let bytes = http::Request::new(url)
        .method(method)
        .send()
        .await?
        .binary()
        .await?;
    Ok(bytes)
}

#[derive(Default)]
struct RootComponent;

enum CoreMessage {
    Message(Event),
    Response(Vec<u8>, Outcome),
}

pub enum Outcome {
    Http(HttpResponse),
}

impl Component for RootComponent {
    type Message = CoreMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(CoreMessage::Message(Event::Get));

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        let reqs = match msg {
            CoreMessage::Message(event) => shared::message(&bcs::to_bytes(&event).unwrap()),
            CoreMessage::Response(uuid, outcome) => shared::response(
                &uuid,
                &match outcome {
                    Outcome::Http(x) => bcs::to_bytes(&x).unwrap(),
                },
            ),
        };

        let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs).unwrap();

        let mut should_render = false;

        for Request { uuid, effect } in reqs {
            match effect {
                Effect::Render => should_render = true,
                Effect::Http(HttpRequest { url, method }) => {
                    let method = match method.as_str() {
                        "GET" => http::Method::GET,
                        "POST" => http::Method::POST,
                        _ => panic!("not yet handling this method"),
                    };

                    let link = link.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        if let Ok(body) = http(&url, method).await {
                            link.send_message(CoreMessage::Response(
                                uuid,
                                Outcome::Http(HttpResponse { status: 200, body }), // TODO: handle status
                            ));
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
        let view: ViewModel = bcs::from_bytes(&view).unwrap();

        html! {
            <>
                <section class="section title has-text-centered">
                    <p>{"Crux Counter Example"}</p>
                </section>
                <section class="section container has-text-centered">
                    <p>{&view.count}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| CoreMessage::Message(Event::Decrement))}>
                        {"Decrement"}
                    </button>
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| CoreMessage::Message(Event::Increment))}>
                        {"Increment"}
                    </button>
                </div>
            </>
        }
    }
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
