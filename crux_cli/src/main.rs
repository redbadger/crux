use anyhow::Result;
use args::Commands;
use clap::Parser;

use args::Cli;

mod args;
mod config;
mod display;
mod doctor;
mod workspace;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Doctor { .. }) => doctor::doctor(&cli.template_dir),
        None => Ok(()),
    }
}
