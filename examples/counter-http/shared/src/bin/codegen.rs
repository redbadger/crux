use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{BoltFfi, Config, PackageLocation, TypeRegistry};
use log::info;

use shared::Counter;

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
    registry.register_app::<Counter>()?;
    // HTTP is `crux_http`'s business, so the shells hold an instance of the
    // handler it ships and delegate to it. Server-Sent Events are this app's
    // own capability, so there is nothing to ship: each shell implements
    // `serverSentEvents` itself.
    registry.shell_handler(&crux_http::HTTP)?;

    let typegen_app = registry
        .build()?
        // Where `boltffi pack` puts the bindings for each shell, so that the
        // generated `Core` can be constructed with nothing but a handler.
        // `boltffi.toml` puts the Kotlin bindings in a package of their own.
        .boltffi(
            BoltFfi::new()
                .swift("Shared")
                .kotlin_package("com.crux.examples.counter.http")
                .typescript("shared", PackageLocation::Path("../pkg".to_string())),
        );

    let name = match args.language {
        Language::Swift => "App",
        Language::Kotlin => "com.crux.examples.counter",
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
