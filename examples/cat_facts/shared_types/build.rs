use std::path::PathBuf;

use crux_core::typegen::TypeGen;

use shared::{
    http::HttpError,
    key_value::{error::KeyValueError, value::Value, KeyValueResponse},
    CatFacts,
};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<CatFacts>()?;

    // types from `crux_http` that aren't automatically discovered
    gen.register_type::<HttpError>()?;

    // types from `crux_kv` that aren't automatically discovered
    gen.register_type::<KeyValueResponse>()?;
    gen.register_type::<KeyValueError>()?;
    gen.register_type::<Value>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
