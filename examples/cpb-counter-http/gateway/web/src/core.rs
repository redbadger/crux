//! The effect loop: like counter-http's Leptos shell, plus `BleConnect` and `BleWrite`.

use std::rc::Rc;

use futures_util::StreamExt;
use leptos::{prelude::*, task};

use cpb_protocol::SseResponse;
use gateway_core::{Effect, Event, Gateway, ViewModel};

use crate::{ble, http, sse};

pub type Core = Rc<crux_core::Core<Gateway>>;

pub fn new() -> Core {
    Rc::new(crux_core::Core::new())
}

pub fn update(core: &Core, event: Event, render: WriteSignal<ViewModel>) {
    log::debug!("event: {event:?}");

    for effect in core.process_event(event) {
        process_effect(core, effect, render);
    }
}

/// Resolve `request` and process what follows; `false` if the core no longer wants
/// answers to it (its command finished or was aborted).
fn resolve<Op: crux_core::capability::Operation>(
    core: &Core,
    request: &mut crux_core::Request<Op>,
    output: Op::Output,
    render: WriteSignal<ViewModel>,
) -> bool
where
    Effect: From<crux_core::Request<Op>>,
{
    match core.resolve(request, output) {
        Ok(effects) => {
            for effect in effects {
                process_effect(core, effect, render);
            }
            true
        }
        Err(error) => {
            log::debug!("resolve: {error:?}");
            false
        }
    }
}

pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    log::debug!("effect: {effect:?}");

    match effect {
        Effect::Render(_) => {
            render.update(|view| *view = core.view());
        }

        Effect::Http(mut request) => {
            let core = core.clone();
            task::spawn_local(async move {
                let response = http::request(&request.operation).await;
                resolve(&core, &mut request, response.into(), render);
            });
        }

        Effect::ServerSentEvents(mut request) => {
            let core = core.clone();
            task::spawn_local(async move {
                let operation = request.operation.clone();
                match sse::request(&operation).await {
                    Ok(stream) => {
                        let mut stream = std::pin::pin!(stream);
                        while let Some(Ok(chunk)) = stream.next().await {
                            // Stops reading (and so closes the fetch) once the core has
                            // aborted the stream, e.g. after the device disconnected.
                            if !resolve(&core, &mut request, chunk, render) {
                                return;
                            }
                        }
                    }
                    Err(error) => log::warn!("SSE {}: {error}", operation.url),
                }
                resolve(&core, &mut request, SseResponse::Done, render);
            });
        }

        Effect::BleConnect(mut request) => {
            let core = core.clone();
            task::spawn_local(async move {
                let mut events = ble::connect(&request.operation);
                while let Some(event) = events.next().await {
                    if !resolve(&core, &mut request, event, render) {
                        return;
                    }
                }
            });
        }

        Effect::BleWrite(mut request) => {
            let core = core.clone();
            task::spawn_local(async move {
                let result = ble::write(&request.operation).await;
                resolve(&core, &mut request, result, render);
            });
        }
    }
}
