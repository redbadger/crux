use std::rc::Rc;

use leptos::prelude::{Update as _, WriteSignal};
use shared::{Counter, Effect, Event, ViewModel};

pub type Core = Rc<shared::Core<Counter>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }
    };
}
