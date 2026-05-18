use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::{
    bindgen::bindgen_kotlin,
    type_generation::facet::{Config, TypeRegistry},
};
use log::info;
use uniffi::deps::anyhow::Result;

use shared::NoteEditor;

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

    let typegen_app = TypeRegistry::new().register_app::<NoteEditor>()?.build()?;

    let name = match args.language {
        Language::Swift => "App",
        Language::Kotlin => "com.crux.example.notes",
        Language::Typescript => "app",
    };
    let config = Config::builder(name, &args.output_dir).build();

    match args.language {
        Language::Swift => {
            info!("Typegen for Swift");
            typegen_app.swift(&config)?;
        }
        Language::Kotlin => {
            info!("Typegen for Kotlin");
            typegen_app.kotlin(&config)?;

            info!("Bindgen for Kotlin");
            bindgen_kotlin(env!("CARGO_PKG_NAME"), &args.output_dir)?;
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
