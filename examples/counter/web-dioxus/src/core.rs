use dioxus::prelude::{to_owned, UnboundedReceiver, UseState};
use futures_util::{StreamExt, TryStreamExt};
use shared::{http::protocol::HttpResult, App, Capabilities, Effect, Event, ViewModel};
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

use crate::{http, sse};

pub type Core = Rc<shared::Core<Effect, App>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new::<Capabilities>())
}

pub async fn core_service(
    core: &Core,
    mut rx: UnboundedReceiver<Event>,
    view: UseState<ViewModel>,
) {
    while let Some(event) = rx.next().await {
        update(core, event, &view);
    }
}

pub fn update(core: &Core, event: Event, view: &UseState<ViewModel>) {
    log::debug!("event: {:?}", event);
    for effect in core.process_event(event) {
        process_effect(core, effect, view);
    }
}

pub fn process_effect(core: &Core, effect: Effect, view: &UseState<ViewModel>) {
    log::debug!("effect: {:?}", effect);
    match effect {
        Effect::Render(_) => {
            view.set(core.view());
        }

        Effect::Http(mut request) => {
            spawn_local({
                to_owned![core, view];
                async move {
                    let response = http::request(&request.operation).await.unwrap();

                    for effect in core.resolve(&mut request, HttpResult::Ok(response)) {
                        process_effect(&core, effect, &view);
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
                            process_effect(&core, effect, &view);
                        }
                    }
                }
            });
        }
    };
}
