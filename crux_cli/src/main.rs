use anyhow::Result;
use args::Commands;
use clap::Parser;

use crate::args::Cli;

mod args;
mod config;
mod workspace;

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Some(Commands::Doctor { .. }) => {
            let workspace = workspace::read_config()?;
            println!("{:#?}", workspace);

            workspace::write_config(&workspace)
        }
        None => Ok(()),
    }
}
