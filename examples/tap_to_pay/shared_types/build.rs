use anyhow::Result;
use crux_core::{bridge::Request, typegen::TypeGen};
use crux_http::protocol::{HttpRequest, HttpResponse};
use shared::{EffectFfi, Event, Payment, PaymentStatus, Receipt, ReceiptStatus, ViewModel};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    register_types(&mut gen).expect("type registration failed");

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))
        .expect("swift type gen failed");
}

fn register_types(gen: &mut TypeGen) -> Result<()> {
    gen.register_type::<Request<EffectFfi>>()?;

    gen.register_type::<EffectFfi>()?;
    gen.register_type::<HttpRequest>()?;

    gen.register_type::<Event>()?;
    gen.register_type::<HttpResponse>()?;

    gen.register_type::<ViewModel>()?;
    gen.register_type::<Payment>()?;
    gen.register_type::<PaymentStatus>()?;
    gen.register_type::<Receipt>()?;
    gen.register_type::<ReceiptStatus>()?;

    Ok(())
}
