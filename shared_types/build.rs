use rmm::{typegen::TypeGen, Request, RequestBody, Response, ResponseBody};
use shared::{platform, Msg, ViewModel};
use std::path::PathBuf;

fn main() {
    let mut gen = TypeGen::new();

    gen.register_type::<Msg>().unwrap();
    gen.register_type::<platform::PlatformMsg>().unwrap();
    gen.register_type::<ViewModel>().unwrap();
    gen.register_type::<Request>().unwrap();
    gen.register_type::<RequestBody>().unwrap();
    gen.register_type::<Response>().unwrap();
    gen.register_type::<ResponseBody>().unwrap();

    let output_root = PathBuf::from("./generated");
    gen.swift(output_root.join("swift"));
    gen.java(output_root.join("java"));
    gen.typescript(output_root.join("typescript"));
}
