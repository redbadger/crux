use std::path::{Path, PathBuf};

use crux_core::{
    cli::BindgenArgs,
    type_generation::facet::{Config, ExternalPackage, PackageLocation, TypeRegistry},
};
use log::info;
use shared::{
    App,
    sse::{SseRequest, SseResponse},
};
use uniffi::deps::anyhow::Result;

fn main() -> Result<()> {
    pretty_env_logger::init();

    let out_dir = PathBuf::from("./shared/generated");

    info!("Generating code for Serde");
    serde(&out_dir.join("serde"))?;

    info!("Generating code for Server Sent Events");
    sse(&out_dir.join("sse"))?;

    info!("Generating code for App");
    app(&out_dir.join("app"))?;

    // bindgen for kotlin
    crux_core::cli::bindgen(&BindgenArgs {
        crate_name: env!("CARGO_PKG_NAME").to_string(),
        out_dir: out_dir.join("app"),
        kotlin: true,
        swift: false,
    })
}

fn app(out_dir: &Path) -> Result<()> {
    let typegen_app = TypeRegistry::new().register_app::<App>().build();

    typegen_app.swift(
        &Config::builder("App", out_dir.join("swift"))
            .reference(ExternalPackage {
                for_namespace: "server_sent_events".to_string(),
                location: PackageLocation::Path("../../sse/swift/ServerSentEvents".to_string()),
                module_name: None,
                version: None,
            })
            .reference(ExternalPackage {
                for_namespace: "serde".to_string(),
                location: PackageLocation::Path("../../serde/swift/Serde".to_string()),
                module_name: None,
                version: None,
            })
            .add_extensions()
            .build(),
    )?;

    typegen_app.kotlin(
        &Config::builder("com.crux.example.counter.app", out_dir.join("kotlin"))
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
    )?;

    typegen_app.typescript(
        &Config::builder("app", out_dir.join("typescript"))
            .reference(ExternalPackage {
                for_namespace: "server_sent_events".to_string(),
                location: PackageLocation::Path("../../sse/typescript".to_string()),
                module_name: Some("server_sent_events".to_string()),
                version: None,
            })
            .reference(ExternalPackage {
                for_namespace: "serde".to_string(),
                location: PackageLocation::Path("../../serde/typescript".to_string()),
                module_name: Some("serde".to_string()),
                version: None,
            })
            .add_extensions()
            .build(),
    )?;

    Ok(())
}

fn sse(out_dir: &Path) -> Result<()> {
    let typegen_sse = TypeRegistry::new()
        .register_type::<SseRequest>()
        .register_type::<SseResponse>()
        .build();

    typegen_sse.swift(
        &Config::builder("ServerSentEvents", out_dir.join("swift"))
            .reference(ExternalPackage {
                for_namespace: "serde".to_string(),
                location: PackageLocation::Path("../../serde/swift/Serde".to_string()),
                module_name: None,
                version: None,
            })
            .build(),
    )?;

    typegen_sse.kotlin(
        &Config::builder("com.crux.example.counter.sse", out_dir.join("kotlin"))
            .reference(ExternalPackage {
                for_namespace: "serde".to_string(),
                location: PackageLocation::Path("com.novi.serde".to_string()),
                module_name: None,
                version: None,
            })
            .build(),
    )?;

    typegen_sse.typescript(
        &Config::builder("server_sent_events", out_dir.join("typescript"))
            .reference(ExternalPackage {
                for_namespace: "serde".to_string(),
                location: PackageLocation::Path("../../serde/typescript".to_string()),
                module_name: Some("serde".to_string()),
                version: None,
            })
            .build(),
    )?;

    Ok(())
}

fn serde(out_dir: &Path) -> Result<()> {
    let typegen_serde = TypeRegistry::new().build();

    typegen_serde.swift(
        &Config::builder("Serde", out_dir.join("swift"))
            .add_runtimes()
            .build(),
    )?;

    typegen_serde.kotlin(
        &Config::builder("com.crux.example.counter.serde", out_dir.join("kotlin"))
            .add_runtimes()
            .build(),
    )?;

    typegen_serde.typescript(
        &Config::builder("serde", out_dir.join("typescript"))
            .add_runtimes()
            .build(),
    )?;

    Ok(())
}
