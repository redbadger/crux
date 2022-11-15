use anyhow::{anyhow, Result};
use js_sys::Date;
use serde::{Deserialize, Serialize};
use shared::*;
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

#[derive(Serialize, Deserialize)]
enum CoreMessage {
    Message(Msg),
    Response(Response),
}

impl Component for HelloWorld {
    type Message = CoreMessage;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(CoreMessage::Message(Msg::Get));
        link.send_message(CoreMessage::Message(Msg::Platform(
            platform::PlatformMsg::Get,
        )));

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        let reqs = match msg {
            CoreMessage::Message(msg) => {
                let msg = bcs::to_bytes(&msg).unwrap();
                shared::message(&msg)
            }
            CoreMessage::Response(resp) => {
                let resp = bcs::to_bytes(&resp).unwrap();
                shared::response(&resp)
            }
        };

        let reqs: Vec<Request> = bcs::from_bytes(&reqs).unwrap();

        reqs.into_iter().any(|req| {
            let Request { uuid, body } = req;
            match body {
                RequestBody::Render => true,
                RequestBody::Time => {
                    link.send_message(CoreMessage::Response(Response {
                        body: ResponseBody::Time(time_get().unwrap()),
                        uuid,
                    }));

                    false
                }
                RequestBody::Http(url) => {
                    let link = link.clone();

                    wasm_bindgen_futures::spawn_local(async move {
                        let bytes = http_get(&url).await.unwrap_or_default();

                        link.send_message(CoreMessage::Response(Response {
                            body: ResponseBody::Http(bytes),
                            uuid,
                        }));
                    });

                    false
                }
                RequestBody::Platform => {
                    link.send_message(CoreMessage::Response(Response {
                        body: ResponseBody::Platform(
                            platform_get().unwrap_or_else(|_| "Unknown browser".to_string()),
                        ),
                        uuid,
                    }));

                    false
                }
                RequestBody::KVRead(_) => {
                    // TODO implement state restoration
                    link.send_message(CoreMessage::Response(Response {
                        body: ResponseBody::KVRead(None),
                        uuid,
                    }));

                    false
                }
                RequestBody::KVWrite(_, _) => {
                    link.send_message(CoreMessage::Response(Response {
                        body: ResponseBody::KVWrite(false),
                        uuid,
                    }));

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
                        onclick={link.callback(|_| CoreMessage::Message(Msg::Clear))}>
                        {"Clear"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| CoreMessage::Message(Msg::Get))}>
                        {"Get"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| CoreMessage::Message(Msg::Fetch))}>
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
