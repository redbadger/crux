mod http;
mod sse;

use std::rc::Rc;

use futures_util::TryStreamExt;
use yew::{html::Scope, prelude::*};

use shared::{App, Capabilities, Core, Effect, Event};

#[derive(Default)]
struct RootComponent {
    core: Rc<Core<Effect, App>>,
}

enum Task {
    Event(Event),
    Effect(Effect),
}

fn send_effects(link: &Scope<RootComponent>, effects: Vec<Effect>) {
    link.send_message_batch(effects.into_iter().map(Task::Effect).collect());
}

impl Component for RootComponent {
    type Message = Task;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link();
        link.send_message(Task::Event(Event::StartWatch));

        Self {
            core: Rc::new(Core::new::<Capabilities>()),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        let link = ctx.link();
        let core = &self.core;

        match msg {
            Task::Event(event) => {
                let effects = core.process_event(event);
                send_effects(link, effects)
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
                Effect::ServerSentEvents(mut request) => {
                    wasm_bindgen_futures::spawn_local({
                        let link = link.clone();
                        let core = core.clone();

                        async move {
                            let mut stream = sse::request(&request.operation).await.unwrap();

                            while let Ok(Some(response)) = stream.try_next().await {
                                let effects = core.resolve(&mut request, response);
                                send_effects(&link, effects);
                            }
                        }
                    });
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
                            onclick={link.callback(|_| Task::Event(Event::Decrement))}>
                            {"Decrement"}
                        </button>
                        <button class="button is-primary is-danger"
                            onclick={link.callback(|_| Task::Event(Event::Increment))}>
                            {"Increment"}
                        </button>
                    </div>
                </section>
            </>
        }
    }
}

fn main() {
    yew::Renderer::<RootComponent>::new().render();
}
