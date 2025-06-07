#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

mod args;
pub mod bindgen;
pub mod codegen;

use anyhow::Result;
use clap::Parser;

pub use args::*;

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Codegen(args) => codegen::codegen(args),
        Commands::Bindgen(args) => bindgen::bindgen(args),
    }
}
