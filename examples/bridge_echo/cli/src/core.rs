use anyhow::{anyhow, Result};
use crossbeam_channel::Sender;
use std::sync::Arc;
use tracing::debug;

use shared::{App, Effect, Event};

pub type Core = Arc<shared::Core<App>>;

pub fn new() -> Core {
    Arc::new(shared::Core::new())
}

pub fn update(core: &Core, event: Event, tx: &Arc<Sender<Effect>>) -> Result<()> {
    debug!("event: {:?}", event);

    for effect in core.process_event(event) {
        process_effect(core, effect, tx)?;
    }
    Ok(())
}

pub fn process_effect(_core: &Core, effect: Effect, tx: &Arc<Sender<Effect>>) -> Result<()> {
    debug!("effect: {:?}", effect);

    match effect {
        render @ Effect::Render(_) => {
            tx.send(render).map_err(|e| anyhow!("{:?}", e))?;
        }
    }
    Ok(())
}
