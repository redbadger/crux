use std::rc::Rc;

use leptos::{prelude::*, task};

use shared::{Effect, Event, ViewModel, Weather};

use crate::{http, kv, location};

pub type Core = Rc<shared::Core<Weather>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {event:?}");

    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }

        Effect::Http(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }

        Effect::KeyValue(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let response = kv::handle(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response)
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }

        Effect::Location(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let response = location::handle(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response)
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }
    }
}
