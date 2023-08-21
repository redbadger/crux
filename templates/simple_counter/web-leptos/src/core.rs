use std::rc::Rc;

use leptos::{SignalUpdate, WriteSignal};
use {{core_name}}::{Capabilities, Core, Counter, Effect, Event, ViewModel};

pub fn new() -> Rc<Core<Effect, Counter>> {
    Rc::new(Core::new::<Capabilities>())
}

pub fn update(core: &Rc<Core<Effect, Counter>>, event: Event, render: WriteSignal<ViewModel>) {
    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(
    core: &Rc<Core<Effect, Counter>>,
    effect: Effect,
    render: WriteSignal<ViewModel>,
) {
    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }
    };
}
