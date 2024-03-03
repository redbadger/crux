use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use crossbeam_channel::Sender;
use std::{sync::Arc, time::SystemTime};
use tokio::{
    fs::{File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
    spawn,
};
use tracing::debug;

use shared::{
    key_value::{KeyValueOperation, KeyValueOutput},
    platform::PlatformResponse,
    time::TimeResponse,
    CatFactCapabilities, CatFacts, Effect, Event,
};

use crate::http;

pub type Core = Arc<shared::Core<Effect, CatFacts>>;

pub fn new() -> Core {
    Arc::new(shared::Core::new::<CatFactCapabilities>())
}

pub fn update(core: &Core, event: Event, tx: &Arc<Sender<Effect>>) -> Result<()> {
    debug!("event: {:?}", event);

    for effect in core.process_event(event) {
        process_effect(core, effect, tx)?;
    }
    Ok(())
}

pub fn process_effect(core: &Core, effect: Effect, tx: &Arc<Sender<Effect>>) -> Result<()> {
    debug!("effect: {:?}", effect);

    match effect {
        render @ Effect::Render(_) => {
            tx.send(render).map_err(|e| anyhow!("{:?}", e))?;
        }

        Effect::Http(mut request) => {
            spawn({
                let core = core.clone();
                let tx = tx.clone();

                async move {
                    let response = http::request(&request.operation).await;

                    for effect in core.resolve(&mut request, response.into()) {
                        process_effect(&core, effect, &tx)?;
                    }
                    Result::<()>::Ok(())
                }
            });
        }

        Effect::KeyValue(mut request) => match request.operation {
            KeyValueOperation::Read(ref key) => {
                spawn({
                    let core = core.clone();
                    let tx = tx.clone();
                    let key = key.clone();

                    async move {
                        let bytes = read_state(&key).await.ok();
                        let response = KeyValueOutput::Read(bytes);

                        for effect in core.resolve(&mut request, response) {
                            process_effect(&core, effect, &tx)?;
                        }
                        Result::<()>::Ok(())
                    }
                });
            }

            KeyValueOperation::Write(ref key, ref value) => {
                spawn({
                    let core = core.clone();
                    let tx = tx.clone();
                    let key = key.clone();
                    let value = value.clone();

                    async move {
                        let success = write_state(&key, &value).await.is_ok();
                        let response = KeyValueOutput::Write(success);

                        for effect in core.resolve(&mut request, response) {
                            process_effect(&core, effect, &tx)?;
                        }
                        Result::<()>::Ok(())
                    }
                });
            }
        },

        Effect::Platform(mut request) => {
            let response = PlatformResponse("cli".to_string());

            for effect in core.resolve(&mut request, response) {
                process_effect(core, effect, tx)?;
            }
        }

        Effect::Time(mut request) => {
            let now: DateTime<Utc> = SystemTime::now().into();
            let response = TimeResponse(now.to_rfc3339());

            for effect in core.resolve(&mut request, response) {
                process_effect(core, effect, tx)?;
            }
        }
    }
    Ok(())
}

async fn write_state(_key: &str, bytes: &[u8]) -> Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(".cat_facts")
        .await?;

    f.write_all(bytes).await?;

    Ok(())
}

async fn read_state(_key: &str) -> Result<Vec<u8>> {
    let mut f = File::open(".cat_facts").await?;
    let mut buf: Vec<u8> = vec![];

    f.read_to_end(&mut buf).await?;

    Ok(buf)
}
