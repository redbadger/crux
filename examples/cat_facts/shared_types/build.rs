use std::{fs, path::PathBuf};

use crux_core::typegen::{State, TypeGen};

use shared::CatFacts;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<CatFacts>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;
    let registry = match &gen.state {
        State::Generating(registry) => registry,
        _ => panic!("registry creation failed"),
    };
    let s = serde_json::to_string_pretty(registry)?;
    fs::write(output_root.join("registry.json"), s)?;

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
