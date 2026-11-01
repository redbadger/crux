use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use crux_core::type_generation::facet::{BoltFfi, Config, PackageLocation, Platform, TypeRegistry};
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
    // The app's operations bring the protocols of `crux_http`, `crux_kv` and
    // `crux_time`. Register the HTTP and timer implementations that those
    // crates ship. Each shell implements the key-value protocol over its own
    // store.
    registry
        .shell_extension(&crux_http::shell::swift::URL_SESSION)?
        .shell_extension(&crux_http::shell::kotlin::URL_CONNECTION)?
        .shell_extension(&crux_http::shell::typescript::FETCH)?
        .shell_extension(&crux_time::shell::swift::TASK)?
        .shell_extension(&crux_time::shell::kotlin::COROUTINE)?
        .shell_extension(&crux_time::shell::typescript::TIMEOUT)?;

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
            info!("Typegen for Kotlin, for Android");
            // Android, not the JVM, so that the package also has the shipped
            // Android files, such as `WebkitCookieJar`.
            typegen_app.generate(Platform::Android, &config)?;
        }
        Language::Typescript => {
            info!("Typegen for TypeScript");
            typegen_app.typescript(&config)?;
        }
    }

    Ok(())
}
