mod core;

use dioxus::prelude::*;
use log::LevelFilter;

use shared::{Event, ViewModel};

use core::CoreService;

#[component]
fn App() -> Element {
    let view = use_signal(ViewModel::default);

    let core = use_coroutine(|mut rx| {
        let svc = CoreService::new(view);
        async move { svc.run(&mut rx).await }
    });

    rsx! {
        main {
            section { class: "section has-text-centered",
                p { class: "is-size-5", "{view().count}" }
                div { class: "buttons section is-centered",
                    button { class:"button is-primary is-danger",
                        onclick: move |_| {
                            core.send(Event::Reset);
                        },
                        "Reset"
                    }
                    button { class:"button is-primary is-success",
                        onclick: move |_| {
                            core.send(Event::Increment);
                        },
                        "Increment"
                    }
                    button { class:"button is-primary is-warning",
                        onclick: move |_| {
                            core.send(Event::Decrement);
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

    launch(App);
}
