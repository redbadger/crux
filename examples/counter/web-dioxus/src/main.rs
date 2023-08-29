mod core;
mod http;
mod sse;

use dioxus::prelude::*;
use dioxus_web::Config;
use log::LevelFilter;

use shared::Event;

use crate::core::Core;

fn app(cx: Scope<Core>) -> Element {
    let core = cx.props;
    let view = use_state(cx, || core.view());
    let dispatcher = use_coroutine(cx, |rx| {
        to_owned![core, view];
        async move { core::core_service(&core, rx, view).await }
    });

    // send initial event
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

fn main() {
    dioxus_logger::init(LevelFilter::Debug).expect("failed to init logger");
    console_error_panic_hook::set_once();

    dioxus_web::launch_with_props(app, core::new(), Config::new());
}
