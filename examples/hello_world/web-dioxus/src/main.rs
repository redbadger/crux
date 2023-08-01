use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_web::Config;
use futures_util::StreamExt;
use log::LevelFilter;

use shared::{Capabilities, Core, Effect, Event, Hello, ViewModel};

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");
    console_error_panic_hook::set_once();

    log::info!("starting app");
    let core = Rc::new(Core::new::<Capabilities>());
    dioxus_web::launch_with_props(app, core, Config::new());
}

fn app(cx: Scope<Rc<Core<Effect, Hello>>>) -> Element {
    let core = cx.props;
    let view = use_state(cx, || core.view());
    let dispatcher = use_coroutine(cx, |rx| {
        to_owned![core, view];
        async move { core_service(rx, view, &core).await }
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

async fn core_service(
    mut rx: UnboundedReceiver<Event>,
    view: UseState<ViewModel>,
    core: &Rc<Core<Effect, Hello>>,
) {
    while let Some(event) = rx.next().await {
        log::info!("event: {:?}", event);
        let effects = core.process_event(event);
        for effect in effects {
            process_effect(effect, &view, core);
        }
    }
}

fn process_effect(effect: Effect, view: &UseState<ViewModel>, core: &Rc<Core<Effect, Hello>>) {
    match effect {
        Effect::Render(_) => {
            view.set(core.view());
        }
    };
}
