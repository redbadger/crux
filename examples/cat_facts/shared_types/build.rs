use anyhow::Result;
use crux_core::{bridge::Request, typegen::TypeGen};
use crux_http::protocol::{HttpRequest, HttpResponse};
use crux_kv::{KeyValueOperation, KeyValueOutput};
use crux_platform::PlatformResponse;
use crux_time::TimeResponse;
use shared::{app::platform::PlatformEvent, EffectFfi, Event, ViewModel};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    register_types(&mut gen).expect("type registration failed");

    let output_root = PathBuf::from("./generated");

    gen.swift("shared_types", output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )
    .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}

fn register_types(gen: &mut TypeGen) -> Result<()> {
    gen.register_type::<Request<EffectFfi>>()?;

    gen.register_type::<EffectFfi>()?;
    gen.register_type::<HttpRequest>()?;
    gen.register_type::<KeyValueOperation>()?;

    gen.register_type::<Event>()?;
    gen.register_type::<HttpResponse>()?;
    gen.register_type::<KeyValueOutput>()?;
    gen.register_type::<TimeResponse>()?;

    gen.register_type::<PlatformEvent>()?;
    gen.register_type::<PlatformResponse>()?;

    gen.register_type::<ViewModel>()?;
    Ok(())
}
