mod args;
pub mod codegen;

use anyhow::Result;
use args::{Cli, Commands};
use clap::Parser;

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Codegen(args) => codegen::codegen(args),
    }
}
