use std::rc::Rc;

use leptos::{prelude::*, task};

use shared::{Effect, Event, NoteEditor, ViewModel};

use crate::{kv, pubsub, time};

pub type Core = Rc<shared::Core<NoteEditor>>;

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

        Effect::Time(request) => {
            time::handle(core, request, render);
        }

        Effect::PubSub(request) => {
            pubsub::handle(core, request, render);
        }
    }
}
