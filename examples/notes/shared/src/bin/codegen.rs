use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::{
    cli::{BindgenArgsBuilder, bindgen},
    type_generation::facet::{Config, TypeRegistry},
};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::Notes;

const PACKAGE: &str = "com.crux.example.notes";

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

    let typegen_app = TypeRegistry::new().register_app::<Notes>()?.build()?;

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
        Language::Kotlin => {
            info!("Typegen for Kotlin");
            typegen_app.kotlin(
                &Config::builder(PACKAGE, &args.output_dir)
                    .add_extensions()
                    .add_runtimes()
                    .build(),
            )?;

            info!("Bindgen for Kotlin");
            bindgen(
                &BindgenArgsBuilder::default()
                    .crate_name(env!("CARGO_PKG_NAME").to_string())
                    .kotlin(&args.output_dir)
                    .build()?,
            )?;
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(
                &Config::builder("app", &args.output_dir)
                    .add_extensions()
                    .add_runtimes()
                    .build(),
            )?;
        }
    }

    Ok(())
}
