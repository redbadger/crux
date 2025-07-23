#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

mod args;
pub mod bindgen;
pub mod codegen;

use anyhow::Result;
use clap::Parser;

use args::{Cli, Commands};

pub use crate::args::BindgenArgs;

pub fn run(crate_name: Option<&str>) -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match cli.command {
        Commands::Codegen(mut args) => {
            if let Some(context) = crate_name {
                args.crate_name = context.to_string();
            }
            codegen::codegen(&args)
        }
        Commands::Bindgen(mut args) => {
            if let Some(context) = crate_name {
                args.crate_name = context.to_string();
            }
            bindgen::bindgen(&args)
        }
    }
}

pub fn bindgen(args: &BindgenArgs) -> Result<()> {
    bindgen::bindgen(args)
}
