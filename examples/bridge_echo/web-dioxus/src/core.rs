use dioxus::prelude::{UnboundedReceiver, UseState};
use futures_util::StreamExt;
use std::rc::Rc;

use shared::{Capabilities, Counter, Effect, Event, ViewModel};

pub type Core = Rc<shared::Core<Effect, Counter>>;

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
    };
}
