use rmm::{Request, Response};
use serde_reflection::{Tracer, TracerConfig};
use shared_types::Msg;
use std::{fs::File, io::Write, path::PathBuf};

fn main() {
    uniffi_build::generate_scaffolding("./src/shared.udl").unwrap();

    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<Msg>().unwrap();
    tracer.trace_simple_type::<Request>().unwrap();
    tracer.trace_simple_type::<Response>().unwrap();
    let registry = tracer.registry().unwrap();

    // Create Swift definitions.
    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bincode]);
    let generator = serde_generate::swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    let out = String::from_utf8_lossy(&source);

    let path = "./generated/shared_types.swift";
    let mut output = File::create(path).unwrap();
    write!(output, "{}", out).unwrap();

    // Create Java definitions.
    let generator = serde_generate::java::CodeGenerator::new(&config);
    generator
        .write_source_files(PathBuf::from("./generated"), &registry)
        .unwrap();
}
