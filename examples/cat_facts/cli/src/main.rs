mod core;
mod http;

use anyhow::Result;
use clap::Parser;
use crossbeam_channel::unbounded;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use shared::{Effect, Event, ViewModel};

use crate::core::update;

#[derive(Parser, Clone)]
enum Command {
    Clear,
    Get,
    Fetch,
}

impl From<Command> for Event {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Clear => Event::Clear,
            Command::Get => Event::Get,
            Command::Fetch => Event::Fetch,
        }
    }
}

/// CLI to get a cat fact
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let format = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(filter)
        .with(format)
        .init();

    let command = Args::parse().cmd;

    let core = core::new();

    run_loop(&core, vec![Event::Restore])?;
    run_loop(&core, vec![Event::GetPlatform, command.into()])?;

    let ViewModel { platform, fact, .. } = core.view();
    println!("platform: {platform}",);
    println!("{fact}",);

    Ok(())
}

fn run_loop(core: &core::Core, events: Vec<Event>) -> Result<()> {
    let (render_tx, render_rx) = unbounded::<Effect>();
    {
        let render_tx = Arc::new(render_tx);
        for event in events {
            update(core, event, &render_tx.clone())?;
        }
    }

    // wait for core to settle,
    // we could process the render effect(s) here
    // but we do it once at the end, instead
    while let Ok(_effect) = render_rx.recv() {}

    Ok(())
}
