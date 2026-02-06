use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::cli::{bindgen, BindgenArgsBuilder};
use log::info;
use uniffi::deps::anyhow::Result;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum)]
    language: Language,
    #[arg(short, long)]
    output_dir: PathBuf,
}

fn main() -> Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    info!("Generating UniFFI bindings");
    let mut builder = BindgenArgsBuilder::default();
    builder.crate_name(env!("CARGO_PKG_NAME").to_string());

    match args.language {
        Language::Kotlin => {
            builder.kotlin(args.output_dir.join("app"));
        }
        Language::Swift => {
            builder.swift(args.output_dir.join("app"));
        }
    }

    bindgen(&builder.build()?)?;

    Ok(())
}
