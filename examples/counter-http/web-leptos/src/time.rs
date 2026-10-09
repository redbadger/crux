use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_timers::callback::Timeout;
use leptos::prelude::*;

use crux_core::Request;
use shared::{ViewModel, time::operation};

use crate::core::{self, Core};

thread_local! {
    /// Live timeouts, so a `ClearTimer` can drop the one it names. Dropping a
    /// `Timeout` cancels it.
    static TIMERS: RefCell<HashMap<usize, Timeout>> = RefCell::new(HashMap::new());
}

/// `NotifyAfter` is answered exactly once, with the id of the timer that
/// fired.
pub fn notify_after(
    core: &Core,
    mut request: Request<operation::NotifyAfter>,
    render: WriteSignal<ViewModel>,
) {
    let operation::NotifyAfter { id, duration } = request.operation;
    let millis = u32::try_from(std::time::Duration::from(duration).as_millis()).unwrap_or(u32::MAX);

    let core = Rc::clone(core);
    let timeout = Timeout::new(millis, move || {
        TIMERS.with_borrow_mut(|timers| timers.remove(&id.0));
        core::resolve_effect(&core, &mut request, id, render);
    });

    TIMERS.with_borrow_mut(|timers| timers.insert(id.0, timeout));
}

/// `ClearTimer` is a request: drop the timer and answer with the id it named.
///
/// Dropping the [`Timeout`] cancels it, so the `NotifyAfter` request it holds
/// is never resolved.
pub fn clear(
    core: &Core,
    mut request: Request<operation::ClearTimer>,
    render: WriteSignal<ViewModel>,
) {
    let id = request.operation.id;
    TIMERS.with_borrow_mut(|timers| timers.remove(&id.0));

    core::resolve_effect(core, &mut request, id, render);
}
