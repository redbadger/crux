mod args;
pub mod codegen;
mod config;
mod diff;
pub mod doctor;
mod template;
mod workspace;

use anyhow::Result;
use args::{Cli, Commands, DoctorArgs};
use clap::Parser;

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Doctor(DoctorArgs {
            fix: _,
            include_source_code,
            template_dir,
            path,
        }) => doctor::doctor(
            template_dir,
            path.as_deref(),
            cli.verbose,
            *include_source_code,
        ),
        Commands::Codegen(args) => codegen::codegen(args),
    }
}
