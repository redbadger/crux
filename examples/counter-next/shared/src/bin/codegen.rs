use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use crux_core::{
    cli::{BindgenArgsBuilder, bindgen},
    type_generation::facet::{
        Config, ExternalPackage, PackageLocation, TypeGenError, TypeRegistry,
    },
};
use log::info;
use shared::{
    Counter,
    sse::{SseRequest, SseResponse},
};
use uniffi::deps::anyhow::Result;

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

    info!("Generating types for Serde");
    serde(&args)?;

    info!("Generating types for Server Sent Events");
    sse(&args)?;

    info!("Generating types for App");
    app(&args)?;

    if args.language == Language::Kotlin {
        // bindgen for kotlin
        bindgen(
            &BindgenArgsBuilder::default()
                .crate_name(env!("CARGO_PKG_NAME").to_string())
                .kotlin(args.output_dir.join("app"))
                .build()?,
        )?;
    }

    Ok(())
}

fn app(args: &Args) -> Result<(), TypeGenError> {
    let typegen = TypeRegistry::new().register_app::<Counter>()?.build()?;
    let out_dir = args.output_dir.join("app");

    match args.language {
        Language::Swift => typegen.swift(
            &Config::builder("App", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "server_sent_events".to_string(),
                    location: PackageLocation::Path("../sse/ServerSentEvents".to_string()),
                    module_name: None,
                    version: None,
                })
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("../serde/Serde".to_string()),
                    module_name: None,
                    version: None,
                })
                .add_extensions()
                .build(),
        ),
        Language::Kotlin => typegen.kotlin(
            &Config::builder("com.crux.example.counter.app", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "server_sent_events".to_string(),
                    location: PackageLocation::Path(
                        "com.crux.example.counter.sse.server_sent_events".to_string(),
                    ),
                    module_name: None,
                    version: None,
                })
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("com.novi.serde".to_string()),
                    module_name: None,
                    version: None,
                })
                .add_extensions()
                .build(),
        ),
        Language::Typescript => typegen.typescript(
            &Config::builder("app", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "server_sent_events".to_string(),
                    location: PackageLocation::Path("../sse".to_string()),
                    module_name: Some("server_sent_events".to_string()),
                    version: None,
                })
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("../serde".to_string()),
                    module_name: Some("serde".to_string()),
                    version: None,
                })
                .add_extensions()
                .build(),
        ),
    }
}

fn sse(args: &Args) -> Result<(), TypeGenError> {
    let typegen_sse = TypeRegistry::new()
        .register_type::<SseRequest>()?
        .register_type::<SseResponse>()?
        .build()?;
    let out_dir = args.output_dir.join("sse");

    match args.language {
        Language::Swift => typegen_sse.swift(
            &Config::builder("ServerSentEvents", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("../serde/Serde".to_string()),
                    module_name: None,
                    version: None,
                })
                .build(),
        ),
        Language::Kotlin => typegen_sse.kotlin(
            &Config::builder("com.crux.example.counter.sse", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("com.novi.serde".to_string()),
                    module_name: None,
                    version: None,
                })
                .build(),
        ),
        Language::Typescript => typegen_sse.typescript(
            &Config::builder("server_sent_events", &out_dir)
                .reference(ExternalPackage {
                    for_namespace: "serde".to_string(),
                    location: PackageLocation::Path("../serde".to_string()),
                    module_name: Some("serde".to_string()),
                    version: None,
                })
                .build(),
        ),
    }
}

fn serde(args: &Args) -> Result<(), TypeGenError> {
    let typegen_serde = TypeRegistry::new().build()?;
    let out_dir = args.output_dir.join("serde");

    match args.language {
        Language::Swift => {
            typegen_serde.swift(&Config::builder("Serde", &out_dir).add_runtimes().build())
        }
        Language::Kotlin => typegen_serde.kotlin(
            &Config::builder("com.crux.example.counter.serde", &out_dir)
                .add_runtimes()
                .build(),
        ),
        Language::Typescript => {
            typegen_serde.typescript(&Config::builder("serde", &out_dir).add_runtimes().build())
        }
    }
}
