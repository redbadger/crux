use anyhow::Result;
use clap::Parser;

mod args;
mod config;
mod workspace;

fn main() -> Result<()> {
    args::CruxCli::parse();

    let workspace = workspace::read_config()?;
    println!("{:#?}", workspace);

    workspace::write_config(&workspace)
}
