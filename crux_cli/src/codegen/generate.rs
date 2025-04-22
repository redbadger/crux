use std::path::{Path, PathBuf};

use log::{debug, info};
use serde_generate::{java, swift, typescript, Encoding, SourceInstaller};
use serde_reflection::Registry;
use std::{
    fs::{self, File},
    io::Write,
};
use thiserror::Error;

pub type Result = std::result::Result<(), TypeGenError>;

#[derive(Error, Debug)]
pub enum TypeGenError {
    #[error("type generation failed: {0}")]
    Generation(String),
    #[error("error writing generated types")]
    Io(#[from] std::io::Error),
    #[error("`pnpm` is needed for TypeScript type generation, but it could not be found in PATH.\nPlease install it from https://pnpm.io/installation")]
    PnpmNotFound(#[source] std::io::Error),
}

/// Generates types for Swift
/// e.g.
/// ```
/// # use crux_cli::codegen::generate;
/// # let registry = serde_reflection::Registry::new();
/// # let output_root = std::env::temp_dir().join("crux_cli_codegen_doctest");
/// generate::swift(&registry, "SharedTypes", output_root.join("swift"))?;
/// # Ok::<(), generate::TypeGenError>(())
/// ```
pub fn swift(registry: &Registry, module_name: &str, path: impl AsRef<Path>) -> Result {
    let path = path.as_ref().join(module_name);

    // remove any existing generated shared types, this ensures that we remove no longer used types
    remove_dir(&path);

    fs::create_dir_all(&path)?;

    info!("Generating Swift types at {}", path.display());

    let installer = swift::Installer::new(path.clone());
    installer
        .install_serde_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;
    installer
        .install_bincode_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;

    let config = serde_generate::CodeGeneratorConfig::new(module_name.to_string())
        .with_encodings(vec![Encoding::Bincode]);

    installer
        .install_module(&config, registry)
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;

    // add bincode deserialization for Vec<Request>
    let mut output = File::create(
        path.join("Sources")
            .join(module_name)
            .join("Requests.swift"),
    )?;

    let requests_path = extensions_path("swift/requests.swift");

    let requests_data = fs::read_to_string(requests_path)?;

    write!(output, "{}", requests_data)?;

    // wrap it all up in a swift package
    let mut output = File::create(path.join("Package.swift"))?;

    let package_path = extensions_path("swift/Package.swift");

    let package_data = fs::read_to_string(package_path)?;

    write!(
        output,
        "{}",
        package_data.replace("SharedTypes", module_name)
    )?;

    Ok(())
}

/// Generates types for Java (for use with Kotlin)
/// e.g.
/// ```
/// # use crux_cli::codegen::generate;
/// # let registry = serde_reflection::Registry::new();
/// # let output_root = std::env::temp_dir().join("crux_cli_codegen_doctest");
/// generate::java(
///     &registry,
///     "com.redbadger.crux_core.shared_types",
///     output_root.join("java"),
/// )?;
/// # Ok::<(), generate::TypeGenError>(())
/// ```
pub fn java(registry: &Registry, package_name: &str, path: impl AsRef<Path>) -> Result {
    let path = path.as_ref();
    fs::create_dir_all(path)?;

    let package_path = package_name.replace('.', "/");

    // remove any existing generated shared types, this ensures that we remove no longer used types
    remove_dir(path.join(&package_path));
    remove_dir(path.join("com/novi/bincode"));
    remove_dir(path.join("com/novi/serde"));

    info!("Generating Java types at {}", path.display());

    let config = serde_generate::CodeGeneratorConfig::new(package_name.to_string())
        .with_encodings(vec![Encoding::Bincode]);

    let installer = java::Installer::new(path.to_path_buf());
    installer
        .install_serde_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;
    installer
        .install_bincode_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;

    installer
        .install_module(&config, registry)
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;

    let requests_path = extensions_path("java/Requests.java");

    let requests_data = fs::read_to_string(requests_path)?;

    let requests = format!("package {package_name};\n\n{}", requests_data);

    fs::write(path.join(package_path).join("Requests.java"), requests)?;

    Ok(())
}

/// Generates types for TypeScript
/// e.g.
/// ```
/// # use crux_cli::codegen::generate;
/// # let registry = serde_reflection::Registry::new();
/// # let output_root = std::env::temp_dir().join("crux_cli_codegen_doctest");
/// generate::typescript(&registry, "shared_types", "0.1.0", output_root.join("typescript"))?;
/// # Ok::<(), generate::TypeGenError>(())
/// ```
pub fn typescript(
    registry: &Registry,
    module_name: &str,
    version: &str,
    path: impl AsRef<Path>,
) -> Result {
    let path = path.as_ref();

    // remove any existing generated shared types, this ensures that we remove no longer used types
    remove_dir(path);

    fs::create_dir_all(path)?;

    info!("Generating TypeScript types at {}", path.display());

    let installer = typescript::Installer::new(path.to_path_buf());
    installer
        .install_serde_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;
    installer
        .install_bincode_runtime()
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;

    let extensions_dir = extensions_path("typescript");
    copy(extensions_dir.as_ref(), path)?;

    let config = serde_generate::CodeGeneratorConfig::new(module_name.to_string())
        .with_encodings(vec![Encoding::Bincode]);

    let generator = serde_generate::typescript::CodeGenerator::new(&config);
    let mut source = Vec::new();
    generator.output(&mut source, registry)?;

    // FIXME fix import paths in generated code which assume running on Deno
    let out = String::from_utf8_lossy(&source)
        .replace(
            "import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';",
            "",
        )
        .replace(".ts'", "'");

    let types_dir = path.join("types");
    fs::create_dir_all(&types_dir)?;

    // write package.json
    let mut package_json = File::create(path.join("package.json"))?;
    write!(
        package_json,
        "{{\"name\": \"{module_name}\", \"version\": \"{version}\"}}"
    )?;

    // add Typescript package using pnpm
    std::process::Command::new("pnpm")
        .current_dir(path)
        .arg("add")
        .arg("typescript")
        .status()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => TypeGenError::PnpmNotFound(e),
            _ => TypeGenError::Io(e),
        })?;

    let mut output = File::create(types_dir.join(format!("{module_name}.ts")))?;
    write!(output, "{out}")?;

    // Install dependencies
    std::process::Command::new("pnpm")
        .current_dir(path)
        .arg("install")
        .status()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => TypeGenError::PnpmNotFound(e),
            _ => TypeGenError::Io(e),
        })?;

    // Build TS code and emit declarations
    std::process::Command::new("pnpm")
        .current_dir(path)
        .arg("exec")
        .arg("tsc")
        .arg("--build")
        .status()
        .map_err(TypeGenError::Io)?;

    Ok(())
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result {
    fs::create_dir_all(&to)?;

    let entries = fs::read_dir(from)?;
    for entry in entries {
        let entry = entry?;

        let to = to.as_ref().to_path_buf().join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(entry.path(), to)?;
        } else {
            fs::copy(entry.path(), to)?;
        };
    }

    Ok(())
}

fn extensions_path(path: impl AsRef<Path>) -> impl AsRef<Path> {
    let path = path.as_ref();
    let custom = PathBuf::from("./typegen_extensions").join(path);
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("typegen_extensions")
        .join(path);

    match custom.try_exists() {
        Ok(true) => custom,
        Ok(false) => default,
        Err(e) => {
            println!("cant check typegen extensions override: {}", e);
            default
        }
    }
}

fn remove_dir(path: impl AsRef<Path>) {
    let path = path.as_ref();
    if path.exists() {
        debug!("Removing directory: {}", path.display());
        fs::remove_dir_all(path).unwrap_or_default();
    }
}
