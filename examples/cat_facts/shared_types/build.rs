use std::path::PathBuf;

use crux_core::typegen::TypeGen;

use shared::{http::HttpError, CatFacts};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<CatFacts>()?;
    gen.register_type::<HttpError>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
