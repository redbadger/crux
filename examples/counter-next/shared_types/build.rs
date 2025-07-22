use crux_core::type_generation::facet::{Config, ExternalPackage, PackageLocation, TypeRegistry};
use shared::{
    App,
    sse::{SseRequest, SseResponse},
};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let output_root = PathBuf::from("./generated");

    let typegen = TypeRegistry::new().register_app::<App>().build();

    typegen.java("com.crux.example.counter.shared", output_root.join("java"))?;

    typegen.typescript("shared_types", output_root.join("typescript"))?;

    let output_dir = output_root.join("swift");

    // Swift Package for shared types
    let config = Config::builder("SharedTypes", &output_dir)
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
    typegen.swift(config)?;

    // Swift Package for ServerSentEvents
    let config = Config::builder("ServerSentEvents", &output_dir)
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
        .swift(config)?;

    // Swift Package for Serde
    let config = Config::builder("Serde", &output_dir).add_runtimes().build();
    TypeRegistry::new().build().swift(config)?;

    Ok(())
}
