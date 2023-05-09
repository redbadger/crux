use crux_core::{bridge::Request, typegen::TypeGen};
use shared::{App, EffectFfi, Event};
use std::path::PathBuf;
use uuid::Uuid;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_type::<Request<EffectFfi>>().expect("register");

    let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
    gen.register_samples(sample_events).expect("register");

    gen.register_app::<App>().expect("register");

    let output_root = PathBuf::from("./generated");

    gen.swift("shared_types", output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java("com.example.counter.shared_types", output_root.join("java"))
        .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}
