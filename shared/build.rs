use serde_reflection::{Tracer, TracerConfig};
use shared_types::Msg;
use std::{fs::File, io::Write};

fn main() {
    uniffi_build::generate_scaffolding("./src/shared.udl").unwrap();
    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<Msg>().unwrap();
    let registry = tracer.registry().unwrap();

    // Create Swift definitions.
    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bincode]);
    let generator = serde_generate::swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    let out = String::from_utf8_lossy(&source);

    let path = "./generated/shared.swift";
    let mut output = File::create(path).unwrap();
    write!(output, "{}", out).unwrap();
}
