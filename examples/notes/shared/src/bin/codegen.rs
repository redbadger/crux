use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{BoltFfi, Config, PackageLocation, TypeRegistry};
use log::info;

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

    let mut registry = TypeRegistry::new();
    registry.register_app::<NoteEditor>()?;
    // The app's operations bring the protocols of `crux_kv` and `crux_time`.
    // Register the timer implementations that `crux_time` ships. Each shell
    // implements the key-value protocol over its own store.
    registry
        .shell_extension(&crux_time::shell::swift::TASK)?
        .shell_extension(&crux_time::shell::kotlin::COROUTINE)?
        .shell_extension(&crux_time::shell::typescript::TIMEOUT)?;

    let typegen_app = registry
        .build()?
        // Only the web shell is built from this example, so only TypeScript
        // names where `boltffi pack` put its bindings.
        .boltffi(BoltFfi::new().typescript("shared", PackageLocation::Path("../pkg".to_string())));

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
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
