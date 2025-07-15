use crux_core::type_generation::facet::{ExternalPackage, PackageLocation, TypeGen};
use shared::{App, ViewModel};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut typegen_app = TypeGen::new();

    typegen_app.register_app::<App>()?;

    let output_root = PathBuf::from("./generated");

    typegen_app.java("com.crux.example.counter.shared", output_root.join("java"))?;

    typegen_app.typescript("shared_types", output_root.join("typescript"))?;

    typegen_app.swift(
        "SharedTypes",
        output_root.join("swift"),
        vec![
            ExternalPackage {
                for_namespace: "view_model".to_string(),
                location: PackageLocation::Path("../ViewModel".to_string()),
                version: None,
            },
            ExternalPackage {
                for_namespace: "Serde".to_string(),
                location: PackageLocation::Path("../Serde".to_string()),
                version: None,
            },
        ],
        false,
        true,
    )?;

    let mut typegen_viewmodel = TypeGen::new();
    typegen_viewmodel.register_type::<ViewModel>()?;
    typegen_viewmodel.swift(
        "ViewModel",
        output_root.join("swift"),
        vec![ExternalPackage {
            for_namespace: "Serde".to_string(),
            location: PackageLocation::Path("../Serde".to_string()),
            version: None,
        }],
        false,
        false,
    )?;

    let mut typegen_serde = TypeGen::new();
    typegen_serde.swift("Serde", output_root.join("swift"), vec![], true, false)?;

    Ok(())
}
