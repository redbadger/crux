#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

mod args;
pub mod bindgen;
pub mod codegen;

use std::fs;

use anyhow::{Context as _, Result};
use camino::Utf8Path;
use clap::Parser;

pub use args::CodegenArgs;
use args::{Cli, Commands};

pub fn run() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    match &cli.command {
        Commands::Codegen(args) => codegen::codegen(args),
        Commands::Bindgen(args) => bindgen::bindgen(args),
    }
}

/// Load TOML from file if the file exists.
fn load_toml_file(source: Option<&Utf8Path>) -> Result<Option<toml::value::Table>> {
    if let Some(source) = source {
        if source.exists() {
            let contents =
                fs::read_to_string(source).with_context(|| format!("read file: {:?}", source))?;
            return Ok(Some(
                toml::de::from_str(&contents)
                    .with_context(|| format!("parse toml: {:?}", source))?,
            ));
        }
    }

    Ok(None)
}
