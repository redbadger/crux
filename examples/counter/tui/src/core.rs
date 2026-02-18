use anyhow::Result;
use futures::TryStreamExt;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::spawn;

use shared::{Counter, Effect, Event};

use crate::{http, sse};

const MAX_LOG_ENTRIES: usize = 200;

// NOTE: Core is shared (via Arc) across async tasks so that spawned HTTP/SSE
// handlers can call `core.resolve()` directly. An alternative design would keep
// Core owned by the main thread and have async tasks send responses back through
// a channel, but the SSE streaming case (where the same request handle is
// resolved repeatedly) makes that awkward without additional complexity.
pub type Core = Arc<shared::Core<Counter>>;
pub type EventLog = Arc<Mutex<VecDeque<String>>>;

/// A shared flag that async tasks set to signal the main loop should re-render.
pub type RenderFlag = Arc<AtomicBool>;

pub fn new() -> Core {
    Arc::new(shared::Core::new())
}

pub fn new_log() -> EventLog {
    Arc::new(Mutex::new(VecDeque::new()))
}

pub fn new_render_flag() -> RenderFlag {
    Arc::new(AtomicBool::new(true))
}

fn log(event_log: &EventLog, message: String) {
    if let Ok(mut log) = event_log.lock() {
        log.push_back(message);
        while log.len() > MAX_LOG_ENTRIES {
            log.pop_front();
        }
    }
}

pub fn update(core: &Core, event: Event, render_flag: &RenderFlag, event_log: &EventLog) {
    log(event_log, format!("→ event:  {event:?}"));

    for effect in core.process_event(event) {
        process_effect(core, effect, render_flag, event_log);
    }
}

pub fn process_effect(core: &Core, effect: Effect, render_flag: &RenderFlag, event_log: &EventLog) {
    log(event_log, format!("← effect: {effect:?}"));

    match effect {
        Effect::Render(_) => {
            render_flag.store(true, Ordering::Release);
        }

        Effect::Http(mut request) => {
            spawn({
                let core = core.clone();
                let render_flag = render_flag.clone();
                let event_log = event_log.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core
                        .resolve(&mut request, response.into())
                        .expect("core should resolve request")
                    {
                        process_effect(&core, effect, &render_flag, &event_log);
                    }
                    Result::<()>::Ok(())
                }
            });
        }

        Effect::ServerSentEvents(mut request) => {
            spawn({
                let core = core.clone();
                let render_flag = render_flag.clone();
                let operation = request.operation.clone();
                let event_log = event_log.clone();

                async move {
                    let mut stream = sse::request(&operation).await?;

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core
                            .resolve(&mut request, response)
                            .expect("core should resolve request")
                        {
                            process_effect(&core, effect, &render_flag, &event_log);
                        }
                    }
                    Result::<()>::Ok(())
                }
            });
        }
    }
}
