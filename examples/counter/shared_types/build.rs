use anyhow::Result;
use crux_core::{bridge::Request, typegen::TypeGen};
use crux_http::protocol::{HttpRequest, HttpResponse};
use shared::{sse::SseResponse, EffectFfi, Event, ViewModel};
use std::path::PathBuf;
use uuid::Uuid;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    register_types(&mut gen).expect("type registration failed");

    let output_root = PathBuf::from("./generated");

    gen.swift("shared_types", output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java("com.example.counter.shared_types", output_root.join("java"))
        .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}

fn register_types(gen: &mut TypeGen) -> Result<()> {
    gen.register_type::<Request<EffectFfi>>()?;

    gen.register_type::<EffectFfi>()?;
    gen.register_type::<HttpRequest>()?;

    let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
    gen.register_type_with_samples(sample_events)?;

    gen.register_type::<HttpResponse>()?;
    gen.register_type::<SseResponse>()?;

    gen.register_type::<ViewModel>()?;
    Ok(())
}
