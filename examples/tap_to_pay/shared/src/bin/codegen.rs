use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{Config, TypeRegistry};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::App;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Language {
    Swift,
    Kotlin,
    Typescript,
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

    let typegen_app = TypeRegistry::new().register_app::<App>()?.build()?;

    match args.language {
        Language::Swift => {
            info!("Typegen for Swift");
            typegen_app.swift(
                &Config::builder("App", &args.output_dir)
                    .add_extensions()
                    .add_runtimes()
                    .build(),
            )?;
        }
        Language::Kotlin | Language::Typescript => (),
    }

    Ok(())
}
