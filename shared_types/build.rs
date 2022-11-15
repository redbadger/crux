use rmm::{Request, RequestBody, Response, ResponseBody};
use serde_generate::test_utils::Runtime;
use serde_reflection::{Tracer, TracerConfig};
use shared::{platform, Msg, ViewModel};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() {
    let extensions_root = Path::new("./extensions");
    let output_root = Path::new("./generated");

    fs::create_dir_all(output_root).unwrap();

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
    fs::create_dir_all(output_dir.clone()).unwrap();

    let mut source = Vec::new();
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::swift::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    // FIXME workaround for odd namespacing behaviour in Swift output
    // which as far as I can tell does not support namespaces in this way
    let out = String::from_utf8_lossy(&source).replace("shared.", "");

    let out = format!(
        "{out}\n\n{}",
        include_str!("./extensions/swift/requests.swift")
    );

    let path = output_dir.join("shared_types.swift");
    let mut output = File::create(path).unwrap();
    write!(output, "{}", out).unwrap();

    // Create Java definitions.
    let output_dir = output_root.join("java");
    fs::create_dir_all(output_dir.clone()).unwrap();

    let config =
        serde_generate::CodeGeneratorConfig::new("com.redbadger.rmm.shared_types".to_string())
            .with_encodings(vec![serde_generate::Encoding::Bcs]);

    let generator = serde_generate::java::CodeGenerator::new(&config);
    generator.write_source_files(output_dir, &registry).unwrap();

    fs::copy(
        "./extensions/java/com/redbadger/rmm/shared_types/Requests.java",
        "./generated/java/com/redbadger/rmm/shared_types/Requests.java",
    )
    .unwrap();

    // Create TypeScript definitions.
    let extensions_dir = extensions_root.join("typescript");
    let output_dir = output_root.join("typescript");
    let mut source = Vec::new();

    // FIXME this should be the actual route, but the runtime is built
    // for Deno, so we patch it heavily in extensions:
    //
    // let installer = typescript::Installer::new(output_dir.clone());
    // installer.install_serde_runtime().unwrap();
    // installer.install_bcs_runtime().unwrap();
    copy(extensions_dir, output_dir.clone()).expect("Could not copy TS runtime");

    let runtime = Runtime::Bcs;
    let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
        .with_serialization(true)
        .with_encodings(vec![runtime.into()]);

    let generator = serde_generate::typescript::CodeGenerator::new(&config);
    generator.output(&mut source, &registry).unwrap();
    // FIXME fix import paths in generated code which assume running on Deno
    let out = String::from_utf8_lossy(&source).replace(".ts'", "'");

    let types_dir = output_dir.join("types");
    fs::create_dir_all(types_dir).unwrap();

    let mut output = File::create(output_dir.join("types/shared.ts")).unwrap();
    write!(output, "{}", out).unwrap();

    // Install dependencies
    std::process::Command::new("pnpm")
        .current_dir(output_dir.clone())
        .arg("install")
        .status()
        .expect("Could not pnpm install");

    // Build TS code and emit declarations
    std::process::Command::new("pnpm")
        .current_dir(output_dir)
        .arg("exec")
        .arg("tsc")
        .arg("--build")
        .status()
        .expect("Could tsc --build");
}

fn copy(from: PathBuf, to: PathBuf) -> io::Result<()> {
    fs::create_dir_all(to.clone())?;

    let entries = fs::read_dir(from)?;
    for entry in entries {
        let entry = entry?;

        if entry.file_type()?.is_dir() {
            copy(entry.path(), to.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), to.join(entry.file_name()))?;
        };
    }

    Ok(())
}
