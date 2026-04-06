mod http;
mod kv;
mod location;
mod secret;
mod time;

use std::rc::Rc;

use leptos::prelude::*;

use shared::{Effect, Event, ViewModel, Weather};

pub type Core = Rc<shared::Core<Weather>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {event:?}");
    process_effects(core, core.process_event(event), render);
}

fn process_effects(core: &Core, effects: Vec<Effect>, render: WriteSignal<ViewModel>) {
    for effect in effects {
        process_effect(core, effect, render);
    }
}

fn resolve_effect<Output>(
    core: &Core,
    request: &mut impl crux_core::Resolvable<Output>,
    output: Output,
    render: WriteSignal<ViewModel>,
) {
    match core.resolve(request, output) {
        Ok(new_effects) => process_effects(core, new_effects, render),
        Err(e) => log::warn!("failed to resolve effect: {e:?}"),
    }
}

fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => render.set(core.view()),
        Effect::Http(request) => http::resolve(core, request, render),
        Effect::KeyValue(request) => kv::resolve(core, request, render),
        Effect::Location(request) => location::resolve(core, request, render),
        Effect::Secret(request) => secret::resolve(core, request, render),
        Effect::Time(request) => time::resolve(core, request, render),
    }
}
