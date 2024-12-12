use std::{fs, path::PathBuf};

use crux_core::typegen::{State, TypeGen};

use shared::{App, PaymentStatus, ReceiptStatus};

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<App>()?;

    gen.register_type::<PaymentStatus>()?;
    gen.register_type::<ReceiptStatus>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    // temporary write of registry.json for debugging codegen
    let registry = match &gen.state {
        State::Generating(registry) => registry,
        _ => panic!("registry creation failed"),
    };
    let s = serde_json::to_string_pretty(registry)?;
    fs::write(output_root.join("registry.json"), s)?;

    Ok(())
}
