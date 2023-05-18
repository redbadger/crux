use crux_core::typegen::TypeGen;
use shared::{app::platform::PlatformEvent, CatFacts};
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<CatFacts>().expect("register");

    // Note: currently required as we can't find enums inside enums, see:
    // https://github.com/zefchain/serde-reflection/tree/main/serde-reflection#supported-features
    gen.register_type::<PlatformEvent>().expect("register");

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java(
        "com.redbadger.catfacts.shared_types",
        output_root.join("java"),
    )
    .expect("java type gen failed");

    gen.typescript("shared_types", output_root.join("typescript"))
        .expect("typescript type gen failed");
}
