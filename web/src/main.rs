use anyhow::Result;
use shared::*;
use web_sys::window;
use woothee::parser::Parser;
use yew::prelude::*;
use yew::use_effect_with_deps;

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

#[function_component(HelloWorld)]
fn hello_world(props: &HelloWorldProps) -> Html {
    struct WebPlatform;
    impl Platform for WebPlatform {
        fn get(&self) -> Result<String, PlatformError> {
            let navigator = window().unwrap().navigator();
            let agent = navigator.user_agent().unwrap_or_default();
            let parser = Parser::new();
            Ok(parser.parse(&agent).unwrap_or_default().name.to_string())
        }
    }
    let result = add_for_platform(1, 2, Box::new(WebPlatform {})).unwrap_or_default();

    let fact = use_state(String::new);
    let bytes = use_state(Vec::<u8>::new);

    if fact.is_empty() {
        let msg = if (*bytes).is_empty() {
            Msg::GetFact
        } else {
            Msg::ReceiveFact {
                bytes: (*bytes).clone(),
            }
        };
        match props.core.update(msg) {
            Cmd::Render { cat_fact } => fact.set(cat_fact),
            Cmd::Get { url } => {
                use_effect_with_deps(
                    move |_| {
                        wasm_bindgen_futures::spawn_local(async move {
                            bytes.set(http_get(&url).await.unwrap_or_default());
                        });
                        || ()
                    },
                    (),
                );
            }
        }
    }

    html! {
        <>
            <section class="section title has-text-centered">
                <p>{result}</p>
            </section>
            <section class="section has-text-centered">
                <p>{(*fact).clone()}</p>
            </section>
        </>
    }
}

fn main() {
    let core = Core::new();

    yew::start_app_with_props::<HelloWorld>(HelloWorldProps { core });
}
