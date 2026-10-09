//! The effect loop: like counter-http's Leptos shell, plus `BleConnect` and `BleWrite`.

use std::rc::Rc;

use futures_util::StreamExt;
use leptos::{prelude::*, task};

use gateway_core::{Effect, Event, Gateway, SseEvent, ViewModel};

use crate::{ble, clock, http, sse};

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
            let view = core.view();
            // Lines first shown now were logged now: the core only changes when the shell
            // hands it an event or an answer, and the shell renders straight after.
            clock::stamp(&view.log);
            render.set(view);
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
                let stream = match sse::request(&request.operation).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        resolve(&core, &mut request, SseEvent::Failed(error), render);
                        return;
                    }
                };
                let mut stream = std::pin::pin!(stream);
                let end = loop {
                    match stream.next().await {
                        Some(Ok(chunk)) => {
                            // Stops reading (and so closes the fetch) once the core has
                            // aborted the stream, e.g. after the device disconnected.
                            if !resolve(&core, &mut request, SseEvent::Chunk(chunk), render) {
                                return;
                            }
                        }
                        Some(Err(error)) => break SseEvent::Failed(error),
                        None => break SseEvent::Closed,
                    }
                };
                resolve(&core, &mut request, end, render);
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
