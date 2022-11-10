use rmm::{Request, RequestBody, Response, ResponseBody};
use serde_reflection::{Tracer, TracerConfig};
use shared::{Msg, PlatformMsg, ViewModel};
use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

fn main() {
    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<Msg>().unwrap();
    tracer.trace_simple_type::<PlatformMsg>().unwrap();
    tracer.trace_simple_type::<ViewModel>().unwrap();
    tracer.trace_simple_type::<Request>().unwrap();
    tracer.trace_simple_type::<RequestBody>().unwrap();
    tracer.trace_simple_type::<Response>().unwrap();
    tracer.trace_simple_type::<ResponseBody>().unwrap();
    let registry = tracer.registry().unwrap();

    // Create Swift definitions.
    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    // FIXME workaround for odd namespacing behaviour in Swift output
    // which as far as I can tell does not support namespaces in this way
    let out = String::from_utf8_lossy(&source).replace("shared.", "");

    let out = format!("{out}\n\n{}", include_str!("./extensions/requests.swift"));

    let path = "./generated/shared_types.swift";
    let mut output = File::create(path).unwrap();
    write!(output, "{}", out).unwrap();

    // Create Java definitions.
    let config =
        serde_generate::CodeGeneratorConfig::new("com.redbadger.rmm.shared_types".to_string())
            .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::java::CodeGenerator::new(&config);
    generator
        .write_source_files(PathBuf::from("./generated"), &registry)
        .unwrap();
    fs::copy(
        "./extensions/com/redbadger/rmm/shared_types/Requests.java",
        "./generated/com/redbadger/rmm/shared_types/Requests.java",
    )
    .unwrap();
}
