mod core;
mod http;
mod sse;

use anyhow::Result;
use clap::Parser;
use crossbeam_channel::unbounded;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use shared::{Effect, Event};

#[derive(Parser, Clone)]
enum Command {
    Get,
    Inc,
    Dec,
    Watch,
}

impl From<Command> for Event {
    fn from(cmd: Command) -> Self {
        match cmd {
            Command::Get => Self::Get,
            Command::Inc => Self::Increment,
            Command::Dec => Self::Decrement,
            Command::Watch => Self::StartWatch,
        }
    }
}

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
    let event = command.into();
    let (tx, rx) = unbounded::<Effect>();

    core::update(&core, event, &tx)?;

    while let Ok(effect) = rx.recv() {
        if let Effect::Render(_) = effect {
            let view = core.view();

            if view.confirmed {
                println!("{text}", text = view.text);
            }
        }
    }

    Ok(())
}
