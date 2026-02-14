use anyhow::{Result, anyhow};
use crossbeam_channel::Sender;
use futures::TryStreamExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::spawn;

use shared::{Counter, Effect, Event};

use crate::{http, sse};

const MAX_LOG_ENTRIES: usize = 200;

pub type Core = Arc<shared::Core<Counter>>;
pub type EventLog = Arc<Mutex<VecDeque<String>>>;

pub fn new() -> Core {
    Arc::new(shared::Core::new())
}

pub fn new_log() -> EventLog {
    Arc::new(Mutex::new(VecDeque::new()))
}

fn log(event_log: &EventLog, message: String) {
    if let Ok(mut log) = event_log.lock() {
        log.push_back(message);
        while log.len() > MAX_LOG_ENTRIES {
            log.pop_front();
        }
    }
}

pub fn update(core: &Core, event: Event, tx: &Sender<Effect>, event_log: &EventLog) -> Result<()> {
    log(event_log, format!("→ event:  {event:?}"));

    for effect in core.process_event(event) {
        process_effect(core, effect, tx, event_log)?;
    }
    Ok(())
}

pub fn process_effect(
    core: &Core,
    effect: Effect,
    tx: &Sender<Effect>,
    event_log: &EventLog,
) -> Result<()> {
    log(event_log, format!("← effect: {effect:?}"));

    match effect {
        render @ Effect::Render(_) => {
            tx.send(render).map_err(|e| anyhow!("{e:?}"))?;
        }

        Effect::Http(mut request) => {
            spawn({
                let core = core.clone();
                let tx = tx.clone();
                let event_log = event_log.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core.resolve(&mut request, response.into())? {
                        process_effect(&core, effect, &tx, &event_log)?;
                    }
                    Result::<()>::Ok(())
                }
            });
        }

        Effect::ServerSentEvents(mut request) => {
            spawn({
                let core = core.clone();
                let tx = tx.clone();
                let operation = request.operation.clone();
                let event_log = event_log.clone();

                async move {
                    let mut stream = sse::request(&operation).await?;

                    while let Ok(Some(response)) = stream.try_next().await {
                        for effect in core.resolve(&mut request, response)? {
                            process_effect(&core, effect, &tx, &event_log)?;
                        }
                    }
                    Result::<()>::Ok(())
                }
            });
        }
    }
    Ok(())
}
