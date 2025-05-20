use crux_core::typegen::TypeGen;
use shared::{App, CurrentResponse};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    // Register the app first
    gen.register_app::<App>()?;

    let _ = gen.register_samples(vec![CurrentResponse::default()]);

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;
    gen.java("com.crux.example.counter", output_root.join("java"))?;
    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
