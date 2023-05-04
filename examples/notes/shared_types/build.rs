use anyhow::Result;
use crux_core::{bridge::Request, typegen::TypeGen};

use shared::{
    capabilities::{
        pub_sub::{Message, PubSubOperation},
        timer::{TimerOperation, TimerOutput},
        KeyValueOperation, KeyValueOutput,
    },
    EffectFfi, Event, TextCursor, ViewModel,
};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    register_types(&mut gen).expect("type registration failed");

    let output_root = PathBuf::from("./generated");

    gen.swift("shared_types", output_root.join("swift"))
        .expect("swift type gen failed");

    // TODO these are for later
    //
    // gen.java("com.example.counter.shared_types", output_root.join("java"))
    //     .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}

fn register_types(gen: &mut TypeGen) -> Result<()> {
    gen.register_type::<Request<EffectFfi>>()?;
    gen.register_type::<EffectFfi>()?;

    gen.register_type::<PubSubOperation>()?;
    gen.register_type::<Message>()?;

    gen.register_type::<TimerOperation>()?;
    gen.register_type::<TimerOutput>()?;

    gen.register_type::<KeyValueOperation>()?;
    gen.register_type::<KeyValueOutput>()?;

    gen.register_type::<Event>()?;
    gen.register_type::<TextCursor>()?;

    gen.register_type::<ViewModel>()?;
    Ok(())
}
