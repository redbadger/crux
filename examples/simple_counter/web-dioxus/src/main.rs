mod core;

use dioxus::prelude::*;
use tracing::Level;

use shared::{Event, ViewModel};

use core::CoreService;

#[component]
fn App() -> Element {
    let view = use_signal(ViewModel::default);

    let core = use_coroutine(move |mut rx| {
        let svc = CoreService::new(view);
        async move { svc.run(&mut rx).await }
    });
    rsx! {
        document::Link {
            rel: "stylesheet",
            href: asset!("./public/css/bulma.min.css")
        }
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
    dioxus_logger::init(Level::DEBUG).expect("failed to init logger");
    console_error_panic_hook::set_once();

    launch(App);
}
