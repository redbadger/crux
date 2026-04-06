use std::rc::Rc;

use leptos::prelude::*;

use crux_time::{TimeRequest, TimeResponse};
use shared::ViewModel;

pub(super) fn resolve(
    core: &super::Core,
    mut request: crux_core::Request<TimeRequest>,
    render: WriteSignal<ViewModel>,
) {
    match &request.operation {
        TimeRequest::Now => {
            log::debug!("time: now");
            let response = TimeResponse::Now {
                instant: now_instant(),
            };
            super::resolve_effect(core, &mut request, response, render);
        }
        TimeRequest::NotifyAfter { id, duration } => {
            let millis =
                u32::try_from(std::time::Duration::from(*duration).as_millis()).unwrap_or(u32::MAX);
            let id = *id;
            log::debug!("time: notify_after {millis}ms (id={id:?})");
            let core = Rc::clone(core);
            wasm_bindgen_futures::spawn_local(async move {
                gloo_timers::future::sleep(std::time::Duration::from_millis(u64::from(millis)))
                    .await;
                log::debug!("time: duration elapsed (id={id:?})");
                let response = TimeResponse::DurationElapsed { id };
                super::resolve_effect(&core, &mut request, response, render);
            });
        }
        TimeRequest::NotifyAt { id, instant } => {
            let target_ms = instant_to_epoch_ms(instant);
            let now_ms = now_epoch_ms();
            let id = *id;
            log::debug!("time: notify_at target={target_ms}ms now={now_ms}ms (id={id:?})");
            let core = Rc::clone(core);
            wasm_bindgen_futures::spawn_local(async move {
                if target_ms > now_ms {
                    let wait = std::time::Duration::from_millis(target_ms - now_ms);
                    gloo_timers::future::sleep(wait).await;
                }
                log::debug!("time: instant arrived (id={id:?})");
                let response = TimeResponse::InstantArrived { id };
                super::resolve_effect(&core, &mut request, response, render);
            });
        }
        TimeRequest::Clear { id } => {
            log::debug!("time: clear (id={id:?})");
            let response = TimeResponse::Cleared { id: *id };
            super::resolve_effect(core, &mut request, response, render);
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn now_epoch_ms() -> u64 {
    js_sys::Date::now().max(0.0) as u64
}

fn now_instant() -> crux_time::Instant {
    let millis = now_epoch_ms();
    let nanos = u32::try_from((millis % 1_000) * 1_000_000).unwrap_or_default();
    crux_time::Instant::new(millis / 1_000, nanos)
}

fn instant_to_epoch_ms(instant: &crux_time::Instant) -> u64 {
    let time: std::time::SystemTime = (*instant).into();
    time.duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or_default()
}
