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

fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,surf=warn".into());
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
    let (tx, rx) = unbounded::<Effect>();
    {
        let tx = Arc::new(tx);
        for event in events {
            update(&core, event, &tx.clone())?;
        }
    }

    // wait for core to settle
    while let Ok(_effect) = rx.recv() {}

    Ok(())
}
