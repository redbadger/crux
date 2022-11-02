use serde_reflection::{Tracer, TracerConfig};
use shared_types::Msg;
// use std::io::Write;

fn main() {
    uniffi_build::generate_scaffolding("./src/shared.udl").unwrap();
    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<Msg>().unwrap();
    let registry = tracer.registry().unwrap();

    // Create Python class definitions.
    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("testing".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bincode]);
    let generator = serde_generate::python3::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
}
