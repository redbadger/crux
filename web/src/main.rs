use anyhow::Result;
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

    fn create(_ctx: &Context<Self>) -> Self {
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

        let cmd = ctx.props().core.update(msg);
        match cmd {
            Cmd::Render { cat_fact } => {
                self.fact = cat_fact;
                true
            }
            Cmd::Get { url } => {
                let link = link.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    link.send_message(Msg::ReceiveFact {
                        bytes: http_get(&url).await.unwrap_or_default(),
                    });
                });
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if self.fact.is_empty() {
            ctx.link().send_message(Msg::GetFact);
        }
        let link = ctx.link();
        let clear = link.callback(|_| Msg::ClearFact);
        let get = link.callback(|_| Msg::GetFact);
        let fetch = link.callback(|_| Msg::FetchFact);
        html! {
            <>
                <section class="section title has-text-centered">
                    <p>{&self.result}</p>
                </section>
                <section class="section has-text-centered">
                    <p>{&self.fact}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-danger" onclick={clear}>{"Clear"}</button>
                    <button class="button is-primary is-success" onclick={get}>{"Get"}</button>
                    <button class="button is-primary is-warning" onclick={fetch}>{"Fetch"}</button>
                </div>
            </>
        }
    }
}

fn main() {
    let core = Core::new();

    yew::start_app_with_props::<HelloWorld>(HelloWorldProps { core });
}
