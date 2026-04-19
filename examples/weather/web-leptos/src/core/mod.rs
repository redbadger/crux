mod http;
mod kv;
mod location;
mod secret;
mod time;

use std::rc::Rc;

use leptos::prelude::*;

use shared::{Effect, Event, ViewModel, Weather};

// ANCHOR: core_base
pub type Core = Rc<shared::Core<Weather>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

/// Push an event into the core and resolve every effect it produces.
pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {event:?}");
    process_effects(core, core.process_event(event), render);
}
// ANCHOR_END: core_base

fn process_effects(core: &Core, effects: Vec<Effect>, render: WriteSignal<ViewModel>) {
    for effect in effects {
        process_effect(core, effect, render);
    }
}

/// Resolve a capability request by handing the response back to the core.
/// The call returns a fresh batch of effects — async commands produce their
/// next step only after the previous one is resolved.
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

// ANCHOR: process_effect
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
// ANCHOR_END: process_effect
