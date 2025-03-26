use crux_core::typegen::TypeGen;
use shared::{App, PaymentStatus, ReceiptStatus};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<App>()?;

    gen.register_type::<PaymentStatus>()?;
    gen.register_type::<ReceiptStatus>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    Ok(())
}
