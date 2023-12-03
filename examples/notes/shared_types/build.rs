use crux_core::typegen::TypeGen;
use shared::{NoteEditor, TextCursor};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("cargo:rerun-if-changed=../shared");

    let mut gen = TypeGen::new();

    gen.register_app::<NoteEditor>()?;

    // Note: currently required as we can't find enums inside enums, see:
    // https://github.com/zefchain/serde-reflection/tree/main/serde-reflection#supported-features
    gen.register_type::<TextCursor>()?;

    let output_root = PathBuf::from("./generated");

    gen.swift("SharedTypes", output_root.join("swift"))?;

    // TODO these are for later
    //
    // gen.java("com.example.counter.shared_types", output_root.join("java"))?;

    gen.typescript("shared_types", output_root.join("typescript"))?;

    Ok(())
}
