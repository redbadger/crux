use crux_core::typegen::TypeGen;
use shared::Counter;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut typegen = TypeGen::new();

    typegen.register_app::<Counter>()?;

    let output_root = PathBuf::from("./generated");

    typegen.swift("SharedTypes", output_root.join("swift"))?;

    typegen.java("com.crux.example.simple_counter", output_root.join("java"))?;

    typegen.typescript("shared_types", output_root.join("typescript"))?;

    typegen.csharp("SharedTypes", output_root.join("csharp"))?;

    Ok(())
}
