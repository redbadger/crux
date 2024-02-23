use std::rc::Rc;

use futures_util::TryStreamExt;
use leptos::{spawn_local, SignalUpdate, WriteSignal};
use shared::{http::protocol::HttpResult, App, Capabilities, Effect, Event, ViewModel};

use crate::{http, sse};

pub type Core = Rc<shared::Core<Effect, App>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new::<Capabilities>())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {:?}", event);

    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    log::debug!("effect: {:?}", effect);

    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }

        Effect::Http(mut request) => {
            spawn_local({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await.unwrap();

                    for effect in core.resolve(&mut request, HttpResult::Ok(response)) {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }

        Effect::ServerSentEvents(mut request) => {
            spawn_local({
                let core = core.clone();

                async move {
                    let mut stream = sse::request(&request.operation).await.unwrap();

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response) {
                            process_effect(&core, effect, render);
                        }
                    }
                }
            });
        }
    };
}
