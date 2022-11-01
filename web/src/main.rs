use anyhow::{anyhow, Result};
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
            Request::Http {
                data: StringEnvelope { uuid, body: url },
            } => {
                let link = link.clone();

                wasm_bindgen_futures::spawn_local(async move {
                    let bytes = http_get(&url).await.unwrap_or_default();

                    link.send_message(CoreMessage::Response(Response::Http {
                        data: BytesEnvelope { body: bytes, uuid },
                    }));
                });

                false
            }
            Request::Platform {
                data: OptionalBoolEnvelope { body: _, uuid },
            } => {
                link.send_message(CoreMessage::Response(Response::Platform {
                    data: StringEnvelope {
                        uuid,
                        body: platform_get().unwrap_or_else(|_| "Unknown browser".to_string()),
                    },
                }));

                false
            }
            Request::Time {
                data: OptionalBoolEnvelope { body: _, uuid },
            } => {
                link.send_message(CoreMessage::Response(Response::Time {
                    data: StringEnvelope {
                        uuid,
                        body: time_get().unwrap(),
                    },
                }));

                false
            }
            Request::KVRead {
                data: StringEnvelope { uuid, body: _key },
            } => {
                // TODO implement state restoration
                link.send_message(CoreMessage::Response(Response::KVRead {
                    data: OptionalBytesEnvelope { uuid, body: None },
                }));

                false
            }
            Request::KVWrite {
                data:
                    KeyValueEnvelope {
                        uuid,
                        body:
                            KeyValue {
                                key: _key,
                                value: _bytes,
                            },
                    },
            } => {
                link.send_message(CoreMessage::Response(Response::KVWrite {
                    data: BoolEnvelope { uuid, body: false },
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
