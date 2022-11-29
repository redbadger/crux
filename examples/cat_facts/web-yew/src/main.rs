use anyhow::{anyhow, Result};
use js_sys::Date;
use shared::{http, key_value, platform, time, Effect, Event, Request, ViewModel};
use web_sys::window;
use woothee::parser::Parser;
use yew::prelude::*;

async fn http_get(url: &str) -> Result<Vec<u8>> {
    let bytes = gloo_net::http::Request::get(url)
        .send()
        .await?
        .binary()
        .await?;
    Ok(bytes)
}

fn time_get() -> Result<String> {
    let date = Date::new_0();

    Ok(format!("{}", date.to_iso_string()))
}

fn platform_get() -> Result<String> {
    let agent = window()
        .ok_or_else(|| anyhow!("no DOM"))?
        .navigator()
        .user_agent()
        .map_err(|e| anyhow!("no user agent {:?}", e))?;

    Ok(Parser::new()
        .parse(&agent)
        .ok_or_else(|| anyhow!("failed to parse user agent"))?
        .name
        .to_string())
}

#[derive(Default)]
struct HelloWorld;

enum CoreMessage {
    Message(Event),
    Response(Vec<u8>, Outcome),
}

pub enum Outcome {
    Platform(platform::Response),
    Time(time::Response),
    Http(http::Response),
    KeyValue(key_value::Response),
}

impl Component for HelloWorld {
    type Message = CoreMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        // link.send_message(CoreMessage::Message(Event::Get));
        link.send_message(CoreMessage::Message(Event::GetPlatform));

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        let reqs = match msg {
            CoreMessage::Message(event) => shared::message(&bcs::to_bytes(&event).unwrap()),
            CoreMessage::Response(uuid, outcome) => shared::response(
                &uuid,
                &match outcome {
                    Outcome::Platform(x) => bcs::to_bytes(&x).unwrap(),
                    Outcome::Time(x) => bcs::to_bytes(&x).unwrap(),
                    Outcome::Http(x) => bcs::to_bytes(&x).unwrap(),
                    Outcome::KeyValue(x) => bcs::to_bytes(&x).unwrap(),
                },
            ),
        };

        let reqs: Vec<Request<Effect>> = bcs::from_bytes(&reqs).unwrap();

        reqs.into_iter().any(|req| {
            let Request { uuid, effect } = req;
            match effect {
                Effect::Render => true,
                Effect::Time => {
                    link.send_message(CoreMessage::Response(
                        uuid,
                        Outcome::Time(time::Response(time_get().unwrap())),
                    ));

                    false
                }
                Effect::Http(http::Request { url, .. }) => {
                    let link = link.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        let bytes = http_get(&url).await.unwrap_or_default();

                        link.send_message(CoreMessage::Response(
                            uuid,
                            Outcome::Http(http::Response {
                                status: 200,
                                body: bytes,
                            }),
                        ));
                    });

                    false
                }
                Effect::Platform => {
                    link.send_message(CoreMessage::Response(
                        uuid,
                        Outcome::Platform(platform::Response(
                            platform_get().unwrap_or_else(|_| "Unknown browser".to_string()),
                        )),
                    ));

                    false
                }
                Effect::KeyValue(key_value::Request::Read(_)) => {
                    // TODO implement state restoration
                    link.send_message(CoreMessage::Response(
                        uuid,
                        Outcome::KeyValue(key_value::Response::Read(None)),
                    ));

                    false
                }
                Effect::KeyValue(key_value::Request::Write(..)) => {
                    link.send_message(CoreMessage::Response(
                        uuid,
                        Outcome::KeyValue(key_value::Response::Write(false)),
                    ));

                    false
                }
            }
        })
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = shared::view();
        let view: ViewModel = bcs::from_bytes(&view).unwrap();

        html! {
            <>
                <section class="section title has-text-centered">
                    <p>{&view.platform}</p>
                </section>
                <section class="section container has-text-centered">
                    if let Some(image) = &view.image {
                        <img src={image.file.clone()} style="height: 400px" />
                    }
                </section>
                <section class="section container has-text-centered">
                    <p>{&view.fact}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| CoreMessage::Message(Event::Clear))}>
                        {"Clear"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| CoreMessage::Message(Event::Get))}>
                        {"Get"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| CoreMessage::Message(Event::Fetch))}>
                        {"Fetch"}
                    </button>
                </div>
            </>
        }
    }
}

fn main() {
    yew::start_app::<HelloWorld>();
}
