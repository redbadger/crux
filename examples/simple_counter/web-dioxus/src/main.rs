mod core;

use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_web::Config;
use log::LevelFilter;

use shared::{Core, Counter, Effect, Event};

fn app(cx: Scope<Rc<Core<Effect, Counter>>>) -> Element {
    let core = cx.props;
    let view = use_state(cx, || core.view());
    let dispatcher = use_coroutine(cx, |rx| {
        to_owned![core, view];
        async move { core::core_service(&core, rx, view).await }
    });

    render! {
        main {
            section { class: "section has-text-centered",
                p { class: "is-size-5", "{view.count}" }
                div { class: "buttons section is-centered",
                    button { class:"button is-primary is-danger",
                        onclick: move |_| {
                            dispatcher.send(Event::Reset);
                        },
                        "Reset"
                    }
                    button { class:"button is-primary is-success",
                        onclick: move |_| {
                            dispatcher.send(Event::Increment);
                        },
                        "Increment"
                    }
                    button { class:"button is-primary is-warning",
                        onclick: move |_| {
                            dispatcher.send(Event::Decrement);
                        },
                        "Decrement"
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
