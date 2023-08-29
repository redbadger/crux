use anyhow::Result;
use args::{Commands, DoctorArgs};
use clap::Parser;

use args::Cli;

mod args;
mod config;
mod diff;
mod doctor;
mod template;
mod workspace;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Doctor(DoctorArgs { .. })) => doctor::doctor(
            &cli.template_dir,
            cli.path.as_deref(),
            cli.verbose,
            cli.include_source_code,
        ),
        None => Ok(()),
    }
}
