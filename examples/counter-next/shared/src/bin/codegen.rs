use std::path::PathBuf;

use crux_core::{
    cli::BindgenArgs,
    type_generation::facet::{Config, ExternalPackage, PackageLocation, TypeRegistry},
};
use shared::{
    App,
    sse::{SseRequest, SseResponse},
};
use uniffi::deps::anyhow;

fn main() -> anyhow::Result<()> {
    let out_dir = PathBuf::from("./shared/generated");

    let typegen = TypeRegistry::new().register_app::<App>().build();

    let config = Config::builder("com.crux.example.counter.shared", out_dir.join("java"))
        .add_extensions()
        .add_runtimes()
        .build();
    typegen.java(&config)?;

    let config = Config::builder("shared_types", out_dir.join("typescript"))
        .add_extensions()
        .add_runtimes()
        .build();
    typegen.typescript(&config)?;

    let output_swift = out_dir.join("swift");

    // Swift Package for shared types
    let config = Config::builder("SharedTypes", &output_swift)
        .reference(ExternalPackage {
            for_namespace: "server_sent_events".to_string(),
            location: PackageLocation::Path("../ServerSentEvents".to_string()),
            version: None,
        })
        .reference(ExternalPackage {
            for_namespace: "Serde".to_string(),
            location: PackageLocation::Path("../Serde".to_string()),
            version: None,
        })
        .add_extensions()
        .build();
    typegen.swift(&config)?;

    // Swift Package for ServerSentEvents
    let config = Config::builder("ServerSentEvents", &output_swift)
        .reference(ExternalPackage {
            for_namespace: "Serde".to_string(),
            location: PackageLocation::Path("../Serde".to_string()),
            version: None,
        })
        .build();
    TypeRegistry::new()
        .register_type::<SseRequest>()
        .register_type::<SseResponse>()
        .build()
        .swift(&config)?;

    // Swift Package for Serde
    let config = Config::builder("Serde", &output_swift)
        .add_runtimes()
        .build();
    TypeRegistry::new().build().swift(&config)?;

    // bindgen for kotlin
    crux_core::cli::bindgen(&BindgenArgs {
        crate_name: env!("CARGO_PKG_NAME").to_string(),
        out_dir,
        kotlin: true,
        swift: false,
    })
}
