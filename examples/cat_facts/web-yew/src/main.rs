mod http;

use std::rc::Rc;

use anyhow::{anyhow, Result};
use js_sys::Date;
use log::info;
use wasm_bindgen::JsValue;
use web_sys::window;
use woothee::parser::Parser;
use yew::{html::Scope, prelude::*};

use shared::{
    key_value::{KeyValueOperation, KeyValueOutput},
    platform::PlatformResponse,
    time::TimeResponse,
    CatFactCapabilities, CatFacts, Core, Effect, Event,
};

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
struct HelloWorld {
    core: Rc<Core<Effect, CatFacts>>,
}

#[derive(Debug)]
enum Task {
    Event(Event),
    Effect(Effect),
}

fn send_effects(link: &Scope<HelloWorld>, effects: Vec<Effect>) {
    link.send_message_batch(effects.into_iter().map(Task::Effect).collect());
}

impl Component for HelloWorld {
    type Message = Task;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(Task::Event(Event::Get));
        link.send_message(Task::Event(Event::GetPlatform));

        Self {
            core: Rc::new(Core::new::<CatFactCapabilities>()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();
        let core = &self.core;

        let object = JsValue::from(format!("message {:?}", msg));
        info!("{}", object.as_string().unwrap());

        match msg {
            Task::Event(event) => {
                send_effects(link, self.core.process_event(event));
            }
            Task::Effect(effect) => match effect {
                Effect::Render(_) => return true,
                Effect::Http(mut request) => {
                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();
                        let core = core.clone();

                        async move {
                            let response = http::request(&request.operation).await.unwrap();

                            let effects = core.resolve(&mut request, response);
                            send_effects(&link, effects);
                        }
                    });
                }
                Effect::KeyValue(mut request) => {
                    let response = match request.operation {
                        KeyValueOperation::Read(_) => KeyValueOutput::Read(None),
                        KeyValueOperation::Write(_, _) => KeyValueOutput::Write(false),
                    };

                    send_effects(link, core.resolve(&mut request, response))
                }
                Effect::Platform(mut request) => {
                    let response = PlatformResponse(
                        platform_get().unwrap_or_else(|_| "Unknown browser".to_string()),
                    );
                    send_effects(link, core.resolve(&mut request, response))
                }
                Effect::Time(mut request) => {
                    let response = TimeResponse(time_get().unwrap());
                    send_effects(link, core.resolve(&mut request, response))
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
                <section class="section title has-text-centered">
                    <p>{&view.platform}</p>
                </section>
                <section class="section container has-text-centered">
                    if let Some(image) = &view.image {
                        <img src={image.href.clone()} style="height: 400px" />
                    }
                </section>
                <section class="section container has-text-centered">
                    <p>{&view.fact}</p>
                </section>
                <div class="buttons container is-centered">
                    <button class="button is-primary is-danger"
                        onclick={link.callback(|_| Task::Event(Event::Clear))}>
                        {"Clear"}
                    </button>
                    <button class="button is-primary is-success"
                        onclick={link.callback(|_| Task::Event(Event::Get))}>
                        {"Get"}
                    </button>
                    <button class="button is-primary is-warning"
                        onclick={link.callback(|_| Task::Event(Event::Fetch))}>
                        {"Fetch"}
                    </button>
                </div>
            </>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<HelloWorld>::new().render();
}
