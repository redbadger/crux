mod core;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use crossbeam_channel::unbounded;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use shared::{Effect, Event};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,surf=warn".into());
    let format = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(filter)
        .with(format)
        .init();

    let core = core::new();
    let (tx, rx) = unbounded::<Effect>();
    let arc_tx = Arc::new(tx);

    tokio::spawn({
        let arc_tx = arc_tx.clone();
        let core = core.clone();

        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                core::update(&core, Event::NewPeriod, &arc_tx).expect("To send an event");
            }
        }
    });

    core::update(&core, Event::Tick, &arc_tx)?;

    while rx.recv().is_ok() {
        let view = core.view();

        if view.count < 1 {
            println!("{text}", text = view.count);
        } else {
            print!("\r{text}", text = view.count);
        }

        core::update(&core, Event::Tick, &arc_tx)?;
    }

    Ok(())
}
