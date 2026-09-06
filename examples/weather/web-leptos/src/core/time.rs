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

/// `Clear` is a notification: drop the timer and answer nothing. The core has
/// already stopped waiting, so resolving the `NotifyAfter` now would be
/// answering a question nobody is asking.
pub(super) fn clear(operation: operation::Clear) {
    log::debug!("time: clear (id={:?})", operation.id);
    TIMERS.with_borrow_mut(|timers| timers.remove(&operation.id.0));
}
