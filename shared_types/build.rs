use anyhow::Result;
use rmm::{typegen::TypeGen, Request, RequestBody, Response, ResponseBody};
use shared::{platform, Msg, ViewModel};
use std::path::PathBuf;

fn main() {
    let mut gen = TypeGen::new();

    register_types(&mut gen).expect("type registration failed");

    let output_root = PathBuf::from("./generated");

    gen.swift(output_root.join("swift"))
        .expect("swift type gen failed");

    gen.java(output_root.join("java"))
        .expect("java type gen failed");

    gen.typescript(output_root.join("typescript"))
        .expect("typescript type gen failed");
}

fn register_types(gen: &mut TypeGen) -> Result<()> {
    gen.register_type::<Msg>()?;
    gen.register_type::<platform::PlatformMsg>()?;
    gen.register_type::<ViewModel>()?;
    gen.register_type::<Request>()?;
    gen.register_type::<RequestBody>()?;
    gen.register_type::<Response>()?;
    gen.register_type::<ResponseBody>()?;
    Ok(())
}
