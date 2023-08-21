use dioxus::prelude::{UnboundedReceiver, UseState};
use futures_util::StreamExt;
use std::rc::Rc;

use {{core_name}}::{Capabilities, Core, Counter, Effect, Event, ViewModel};

pub fn new() -> Rc<Core<Effect, Counter>> {
    Rc::new(Core::new::<Capabilities>())
}

pub async fn core_service(
    core: &Rc<Core<Effect, Counter>>,
    mut rx: UnboundedReceiver<Event>,
    view: UseState<ViewModel>,
) {
    while let Some(event) = rx.next().await {
        update(core, event, &view);
    }
}

pub fn update(core: &Rc<Core<Effect, Counter>>, event: Event, view: &UseState<ViewModel>) {
    log::debug!("event: {:?}", event);
    for effect in core.process_event(event) {
        process_effect(core, effect, view);
    }
}

pub fn process_effect(
    core: &Rc<Core<Effect, Counter>>,
    effect: Effect,
    view: &UseState<ViewModel>,
) {
    log::debug!("effect: {:?}", effect);
    match effect {
        Effect::Render(_) => {
            view.set(core.view());
        }
    };
}
