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

#[derive(Properties, Default, PartialEq)]
pub struct HelloWorldProps {
    pub core: Core,
}

#[derive(Default)]
struct HelloWorld {
    result: String,
    fact: String,
}

impl Component for HelloWorld {
    type Message = Msg;
    type Properties = HelloWorldProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(Msg::GetFact);

        Self::default()
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();

        struct WebPlatform;
        impl Platform for WebPlatform {
            fn get(&self) -> Result<String, PlatformError> {
                let navigator = window().unwrap().navigator();
                let agent = navigator.user_agent().unwrap_or_default();
                let parser = Parser::new();
                Ok(parser.parse(&agent).unwrap_or_default().name.to_string())
            }
        }
        self.result = add_for_platform(1, 2, Box::new(WebPlatform {})).unwrap_or_default();

        match ctx.props().core.update(msg) {
            Cmd::Render => {
                self.fact = ctx.props().core.view().fact;

                true
            }
            Cmd::HttpGet { url } => {
                let link = link.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    link.send_message(Msg::HttpResponse {
                        bytes: http_get(&url).await.unwrap_or_default(),
                    });
                });

                false
            }
            Cmd::TimeGet => {
                link.send_message(Msg::CurrentTime {
                    iso_time: time_get().unwrap(),
                });

                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <>
                <section class="section title has-text-centered">
                    <p>{&self.result}</p>
                </section>
                <section class="section has-text-centered">
                    <p>{&self.fact}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| Msg::ClearFact)}>
                        {"Clear"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| Msg::GetFact)}>
                        {"Get"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| Msg::FetchFact)}>
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
