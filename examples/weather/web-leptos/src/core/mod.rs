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
/// One arm per operation. A Rust shell can match the `Effect` enum directly —
/// each variant carries its operation type, and the compiler knows which
/// output that request has to be resolved with. `Render` and `TimeClear` are
/// notifications, so they are never resolved at all.
fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => render.set(core.view()),
        Effect::Http(request) => http::resolve(core, request, render),
        Effect::KvGet(request) => kv::get(core, request, render),
        Effect::KvSet(request) => kv::set(core, request, render),
        Effect::TimeNotifyAfter(request) => time::notify_after(core, request, render),
        Effect::TimeClear(request) => time::clear(request.operation),
        Effect::IsLocationEnabled(request) => location::is_location_enabled(core, request, render),
        Effect::GetLocation(request) => location::get_location(core, request, render),
        Effect::FetchSecret(request) => secret::fetch(core, request, render),
        Effect::StoreSecret(request) => secret::store(core, request, render),
        Effect::DeleteSecret(request) => secret::delete(core, request, render),
    }
}
// ANCHOR_END: process_effect
