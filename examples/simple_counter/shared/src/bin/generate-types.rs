use crux_core::typegen::TypeGen;
use shared::Counter;
use std::path::PathBuf;
use uniffi::deps::anyhow;

#[cfg(feature = "typegen")]
fn main() -> anyhow::Result<()> {
    let mut gen = TypeGen::new();

    gen.register_app::<Counter>()?;

    let output_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.example.simple_counter.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
