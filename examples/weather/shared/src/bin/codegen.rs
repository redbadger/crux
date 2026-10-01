use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{BoltFfi, Config, PackageLocation, TypeRegistry};
use log::info;

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
    registry.register_app::<shared::Weather>()?;
    // The HTTP client, the store and the timer table are the same in every
    // shell, so the shells hold an instance of what these emit and delegate to
    // it rather than writing the protocols' rules out three times. Registering
    // a handler also registers the types its sources name, so the operations
    // this app never sends — `DeleteValue`, `Now` and the rest — are generated
    // too: the shipped source implements the whole capability.
    registry
        .shell_handler(&crux_http::HTTP)?
        .shell_handler(&crux_kv::KEY_VALUE)?
        .shell_handler(&crux_time::TIME)?;

    let typegen_app = registry
        .build()?
        // Where `boltffi pack` puts the bindings for each shell, so that the
        // generated `Core` can be constructed with nothing but a handler.
        .boltffi(
            BoltFfi::new()
                .swift("Shared")
                .kotlin()
                .typescript("shared", PackageLocation::Path("../pkg".to_string())),
        );

    let name = match args.language {
        Language::Swift => "App",
        Language::Kotlin => "com.crux.example.weather",
        Language::Typescript => "app",
    };
    let mut builder = Config::builder(name, &args.output_dir);
    if args.language == Language::Swift {
        // The BoltFFI package the generated one now depends on declares these,
        // and SPM will not link a package with a lower deployment target.
        builder.platform(".iOS(.v16)").platform(".macOS(.v13)");
    }
    let config = builder.build();

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
