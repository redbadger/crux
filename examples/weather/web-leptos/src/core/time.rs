use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gloo_timers::callback::Timeout;
use leptos::prelude::*;

use crux_core::Request;
use crux_time::operation;
use shared::ViewModel;

thread_local! {
    /// Live timeouts, so a `Clear` can drop the one it names. Dropping a
    /// `Timeout` cancels it.
    static TIMERS: RefCell<HashMap<usize, Timeout>> = RefCell::new(HashMap::new());
}

/// `NotifyAfter` is answered exactly once, with the id of the timer that
/// fired.
pub(super) fn notify_after(
    core: &super::Core,
    mut request: Request<operation::NotifyAfter>,
    render: WriteSignal<ViewModel>,
) {
    let operation::NotifyAfter { id, duration } = request.operation;
    let millis = u32::try_from(std::time::Duration::from(duration).as_millis()).unwrap_or(u32::MAX);
    log::debug!("time: notify_after {millis}ms (id={id:?})");

    let core = Rc::clone(core);
    let timeout = Timeout::new(millis, move || {
        TIMERS.with_borrow_mut(|timers| timers.remove(&id.0));
        log::debug!("time: duration elapsed (id={id:?})");
        super::resolve_effect(&core, &mut request, id, render);
    });

    TIMERS.with_borrow_mut(|timers| timers.insert(id.0, timeout));
}

/// `Clear` is a request: drop the timer and answer with the id it named.
///
/// Dropping the [`Timeout`] cancels it, so the `NotifyAfter` request it holds
/// is never resolved. Resolving it late would be harmless too — the core stops
/// listening for that request the moment it clears the timer.
pub(super) fn clear(
    core: &super::Core,
    mut request: Request<operation::Clear>,
    render: WriteSignal<ViewModel>,
) {
    let id = request.operation.id;
    log::debug!("time: clear (id={id:?})");
    TIMERS.with_borrow_mut(|timers| timers.remove(&id.0));

    super::resolve_effect(core, &mut request, id, render);
}
