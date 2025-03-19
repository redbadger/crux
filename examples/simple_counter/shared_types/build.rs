use crux_core::{bridge::Request, typegen::TypeGen};
use shared::{Counter, EffectFfi};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_type::<EffectFfi>()?;
    gen.register_type::<Request<EffectFfi>>()?;

    gen.register_app::<Counter>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.example.simple_counter.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
