use std::{cell::RefCell, collections::HashSet};

use crux_time::{TimeRequest, TimeResponse};
use leptos::{prelude::*, task};
use wasm_bindgen_futures::JsFuture;

use shared::ViewModel;

use crate::core::{self, Core};

thread_local! {
    static CLEARED: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

#[allow(clippy::future_not_send)] // WASM is single-threaded
pub fn handle(
    core: &Core,
    mut request: crux_core::Request<TimeRequest>,
    render: WriteSignal<ViewModel>,
) {
    match &request.operation {
        TimeRequest::NotifyAfter { id, duration } => {
            let timer_id = id.0;
            let std_duration: std::time::Duration = (*duration).into();
            #[allow(clippy::cast_possible_truncation)]
            let millis = std_duration.as_millis() as i32;
            let id = *id;

            let core = core.clone();

            task::spawn_local(async move {
                let promise = js_sys::Promise::new(&mut |resolve, _| {
                    let window = web_sys::window().expect("should have window");
                    let _ = window
                        .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, millis);
                });
                let _ = JsFuture::from(promise).await;

                let was_cleared = CLEARED.with(|c| c.borrow_mut().remove(&timer_id));
                if was_cleared {
                    return;
                }

                let response = TimeResponse::DurationElapsed { id };
                for effect in core
                    .resolve(&mut request, response)
                    .expect("should resolve")
                {
                    core::process_effect(&core, effect, render);
                }
            });
        }
        TimeRequest::Clear { id } => {
            CLEARED.with(|c| c.borrow_mut().insert(id.0));
        }
        _ => {
            log::warn!("Unhandled time request: {:?}", request.operation);
        }
    }
}
