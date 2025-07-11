use crux_core::type_generation::facet::{ExternalPackage, TypeGen};
use shared::{App, ViewModel};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut typegen = TypeGen::new();

    typegen.register_app::<App>()?;

    let output_root = PathBuf::from("./generated");

    // typegen.java("com.crux.example.counter.shared", output_root.join("java"))?;

    // typegen.typescript("shared_types", output_root.join("typescript"))?;

    typegen.swift(
        "SharedTypes",
        output_root.join("swift"),
        vec![ExternalPackage {
            for_namespace: "view_model".to_string(),
            location: "file://../ViewModel".to_string(),
            version: None,
        }],
        true,
    )?;

    let mut typegen = TypeGen::new();
    typegen.register_type::<ViewModel>()?;
    typegen.swift("ViewModel", output_root.join("swift"), vec![], false)?;

    Ok(())
}
