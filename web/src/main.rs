use anyhow::Result;
use js_sys::Date;
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

fn platform_get() -> String {
    let navigator = window().unwrap().navigator();
    let agent = navigator.user_agent().unwrap_or_default();
    let parser = Parser::new();
    let platform = parser.parse(&agent).unwrap_or_default().name.to_string();
    println!("{platform}");
    platform
}

#[derive(Properties, Default, PartialEq)]
pub struct HelloWorldProps {
    pub core: Core,
}

#[derive(Default)]
struct HelloWorld;

enum CoreMessage {
    Message(Msg),
    Response(Response),
}

impl Component for HelloWorld {
    type Message = CoreMessage;
    type Properties = HelloWorldProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(CoreMessage::Message(Msg::Get));
        link.send_message(CoreMessage::Message(Msg::GetPlatform));

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        let reqs = match msg {
            CoreMessage::Message(msg) => ctx.props().core.message(msg),
            CoreMessage::Response(resp) => ctx.props().core.response(resp),
        };

        reqs.into_iter().any(|req| match req {
            Request::Render => true,
            Request::Http { url, uuid } => {
                let link = link.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let bytes = http_get(&url).await.unwrap_or_default();

                    link.send_message(CoreMessage::Response(Response::Http { uuid, bytes }));
                });

                false
            }
            Request::Platform { uuid } => {
                link.send_message(CoreMessage::Response(Response::Platform {
                    uuid,
                    platform: platform_get(),
                }));

                false
            }
            Request::Time { uuid } => {
                link.send_message(CoreMessage::Response(Response::Time {
                    uuid,
                    iso_time: time_get().unwrap(),
                }));

                false
            }
            Request::KVRead { uuid, key: _key } => {
                // TODO implement state restoration
                link.send_message(CoreMessage::Response(Response::KVRead {
                    uuid,
                    bytes: None,
                }));

                false
            }
            Request::KVWrite {
                uuid,
                key: _key,
                bytes: _bytes,
            } => {
                link.send_message(CoreMessage::Response(Response::KVWrite {
                    uuid,
                    success: false,
                }));

                false
            }
        })
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let view = ctx.props().core.view();

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
    let core = Core::new();

    yew::start_app_with_props::<HelloWorld>(HelloWorldProps { core });
}
