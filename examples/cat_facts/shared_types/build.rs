use std::path::PathBuf;

use crux_core::typegen::TypeGen;

use shared::CatFacts;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    shared::register_effects(&mut gen)?;
    gen.register_app::<CatFacts>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
