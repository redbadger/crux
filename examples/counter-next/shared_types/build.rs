use crux_core::type_generation::facet::TypeGen;
use shared::App;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut typegen = TypeGen::new();

    typegen.register_app::<App>()?;

    let output_root = PathBuf::from("./generated");

    // typegen.swift("SharedTypes", output_root.join("swift"))?;

    // typegen.java("com.crux.example.counter.shared", output_root.join("java"))?;

    typegen.typescript("shared_types", output_root.join("typescript"))?;
    panic!();
    Ok(())
}
