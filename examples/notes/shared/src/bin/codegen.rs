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
    // The store and the timer table are the capabilities' own business, so the
    // shell holds an instance of what these emit and delegates to it rather
    // than writing the protocols' rules out itself. Registering a handler also
    // registers the types its sources name, so the operations this app never
    // sends — `DeleteValue`, `KeyExists`, `ListKeys`, `Now` and `NotifyAt` —
    // are generated too: the shipped source implements the whole capability.
    registry
        .shell_handler(&crux_kv::KEY_VALUE)?
        .shell_handler(&crux_time::TIME)?;

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
