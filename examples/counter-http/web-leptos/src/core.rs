use std::rc::Rc;

use futures_util::TryStreamExt;
use leptos::{prelude::*, task};

use shared::{Counter, Effect, Event, ViewModel, sse::SseResponse};

use crate::{http, sse, time};

pub type Core = Rc<shared::Core<Counter>>;

pub fn new() -> Core {
    Rc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {event:?}");

    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    log::debug!("effect: {effect:?}");

    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }

        Effect::Http(mut request) => {
            task::spawn_local({
                let core = core.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("should resolve")
                    {
                        process_effect(&core, effect, render);
                    }
                }
            });
        }

        Effect::ServerSentEvents(mut request) => {
            task::spawn_local({
                let core = core.clone();
                let operation = request.operation.clone();

                async move {
                    if let Ok(mut stream) = sse::request(&operation).await {
                        while let Ok(Some(response)) = stream.try_next().await {
                            resolve_effect(&core, &mut request, response, render);
                        }
                    }
                    // the stream ended, or failed: say so, so the core can reopen it
                    resolve_effect(&core, &mut request, SseResponse::Done, render);
                }
            });
        }

        Effect::TimeNotifyAfter(request) => time::notify_after(core, request, render),
        Effect::TimeClear(request) => time::clear(core, request, render),
    }
}

/// Resolve a request by handing its output back to the core, and process the
/// effects that follow.
pub fn resolve_effect<Output>(
    core: &Core,
    request: &mut impl crux_core::Resolvable<Output>,
    output: Output,
    render: WriteSignal<ViewModel>,
) {
    match core.resolve(request, output) {
        Ok(effects) => {
            for effect in effects {
                process_effect(core, effect, render);
            }
        }
        Err(e) => log::warn!("failed to resolve effect: {e:?}"),
    }
}
