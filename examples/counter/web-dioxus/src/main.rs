mod http;
mod sse;

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_web::Config;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use log::LevelFilter;
use wasm_bindgen_futures::spawn_local;

use shared::{App, Capabilities, Core, Effect, Event, ViewModel};

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();

    log::info!("starting app");
    let core = Rc::new(Core::new::<Capabilities>());
    dioxus_web::launch_with_props(app, core, Config::new());
}

fn app(cx: Scope<Rc<Core<Effect, App>>>) -> Element {
    let core = cx.props;
    let view = use_state(cx, || core.view());
    let dispatcher = use_coroutine(cx, |rx| {
        to_owned![core, view];
        async move { core_service(rx, view, &core).await }
    });
    use_future(cx, (), |_| {
        to_owned![dispatcher];
        async move { dispatcher.send(Event::StartWatch) }
    });

    render! {
        main {
            section { class: "section has-text-centered",
                h1 { class: "title", "Crux Counter Example" }
                p { class: "is-size-5", "Rust Core, Rust Shell (Dioxus)" }
            }
            section { class: "section has-text-centered",
                p { class: "is-size-5", "{view.text}" }
                div { class: "buttons section is-centered",
                    button { class:"button is-primary is-warning",
                        onclick: move |_| {
                            dispatcher.send(Event::Decrement);
                        },
                        "Decrement"
                    }
                    button { class:"button is-primary is-danger",
                        onclick: move |_| {
                            dispatcher.send(Event::Increment);
                        },
                        "Increment"
                    }
                }
            }
        }
    }
}

async fn core_service(
    mut rx: UnboundedReceiver<Event>,
    view: UseState<ViewModel>,
    core: &Rc<Core<Effect, App>>,
) {
    while let Some(event) = rx.next().await {
        log::info!("event: {:?}", event);
        let effects = core.process_event(event);
        for effect in effects {
            process_effect(effect, &view, core);
        }
    }
}

fn process_effect(effect: Effect, view: &UseState<ViewModel>, core: &Rc<Core<Effect, App>>) {
    match effect {
        Effect::Render(_) => {
            view.set(core.view());
        }
        Effect::Http(mut request) => {
            spawn_local({
                to_owned![core, view];
                async move {
                    let response = http::request(&request.operation).await.unwrap();
                    for effect in core.resolve(&mut request, response) {
                        process_effect(effect, &view, &core);
                    }
                }
            });
        }
        Effect::ServerSentEvents(mut request) => {
            spawn_local({
                to_owned![core, view];
                async move {
                    let mut stream = sse::request(&request.operation).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response) {
                            process_effect(effect, &view, &core);
                        }
                    }
                }
            });
        }
    };
}
