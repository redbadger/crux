use rmm::{Request, RequestBody, Response, ResponseBody};
use serde_generate::{test_utils::Runtime, typescript, SourceInstaller};
use serde_reflection::{Tracer, TracerConfig};
use shared::{platform, Msg, ViewModel};
use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

fn main() {
    let output_root = Path::new("./generated");

    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<Msg>().unwrap();
    tracer.trace_simple_type::<platform::PlatformMsg>().unwrap();
    tracer.trace_simple_type::<ViewModel>().unwrap();
    tracer.trace_simple_type::<Request>().unwrap();
    tracer.trace_simple_type::<RequestBody>().unwrap();
    tracer.trace_simple_type::<Response>().unwrap();
    tracer.trace_simple_type::<ResponseBody>().unwrap();
    let registry = tracer.registry().unwrap();

    // Create Swift definitions.
    let output_dir = output_root.join("swift");
    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    // FIXME workaround for odd namespacing behaviour in Swift output
    // which as far as I can tell does not support namespaces in this way
    let out = String::from_utf8_lossy(&source).replace("shared.", "");

    let out = format!("{out}\n\n{}", include_str!("./extensions/requests.swift"));

    let path = output_dir.join("shared_types.swift");
    let mut output = File::create(path).unwrap();
    write!(output, "{}", out).unwrap();

    // Create Java definitions.
    let output_dir = output_root.join("java");
    let config =
        serde_generate::CodeGeneratorConfig::new("com.redbadger.rmm.shared_types".to_string())
            .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::java::CodeGenerator::new(&config);
    generator.write_source_files(output_dir, &registry).unwrap();
    fs::copy(
        "./extensions/com/redbadger/rmm/shared_types/Requests.java",
        "./generated/java/com/redbadger/rmm/shared_types/Requests.java",
    )
    .unwrap();

    // Create TypeScript definitions.
    let output_dir = output_root.join("typescript");
    let mut source = Vec::new();

    // let installer = typescript::Installer::new(output_dir.clone());
    // installer.install_serde_runtime().unwrap();
    // installer.install_bcs_runtime().unwrap();

    let runtime = Runtime::Bcs;
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_serialization(true)
        .with_encodings(vec![runtime.into()]);

    let generator = serde_generate::typescript::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    // FIXME workaround for odd namespacing behaviour in Swift output
    // which as far as I can tell does not support namespaces in this way
    let out = String::from_utf8_lossy(&source).replace("shared.", "");

    let mut output = File::create(output_dir.join("types/shared.ts")).unwrap();
    write!(output, "{}", out).unwrap();
}
