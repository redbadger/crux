//! Generation of foreign language types (currently Swift, Java, TypeScript) for Crux
//!
//! In order to use this module, you'll need a separate crate from your shared library, possibly
//! called `shared_types`. This is necessary because we need to reference types from your shared library
//! during the build process (`build.rs`).
//!
//! This module is behind the feature called `typegen`, and is not compiled into the default crate.
//!
//! Ensure that you have the following line in the `Cargo.toml` of your `shared_types` library.
//!
//! ```rust,ignore
//! [build-dependencies]
//! crux_core = { version = "0.6", features = ["typegen"] }
//! ```
//!
//! * Your `shared_types` library, will have an empty `lib.rs`, since we only use it for generating foreign language type declarations.
//! * Create a `build.rs` in your `shared_types` library, that looks something like this:
//!
//! ```rust
//! # mod shared {
//! #     use crux_core::render::Render;
//! #     use crux_macros::Effect;
//! #     use serde::{Deserialize, Serialize};
//! #     #[derive(Default)]
//! #     pub struct App;
//! #     #[derive(Serialize, Deserialize)]
//! #     pub enum Event {
//! #         None,
//! #         SendUuid(uuid::Uuid),
//! #     }
//! #     #[derive(Serialize, Deserialize)]
//! #     pub struct ViewModel;
//! #     impl crux_core::App for App {
//! #         type Event = Event;
//! #         type Model = ();
//! #         type ViewModel = ViewModel;
//! #         type Capabilities = Capabilities;
//! #         fn update(&self, _event: Event, _model: &mut Self::Model, _caps: &Capabilities) {}
//! #         fn view(&self, _model: &Self::Model) -> Self::ViewModel {
//! #             todo!();
//! #         }
//! #     }
//! #     #[derive(Effect)]
//! #     pub struct Capabilities {
//! #         pub render: Render<Event>,
//! #     }
//! # }
//!use shared::{App, EffectFfi, Event};
//!use crux_core::{bridge::Request, typegen::TypeGen};
//!use uuid::Uuid;
//!
//!#[test]
//!fn generate_types() {
//!    let mut gen = TypeGen::new();
//!
//!    let sample_events = vec![Event::SendUuid(Uuid::new_v4())];
//!    gen.register_type_with_samples(sample_events).unwrap();
//!
//!    gen.register_app::<App>().unwrap();
//!
//!    let temp = assert_fs::TempDir::new().unwrap();
//!    let output_root = temp.join("crux_core_typegen_test");
//!
//!    gen.swift("SharedTypes", output_root.join("swift"))
//!        .expect("swift type gen failed");
//!
//!    gen.java("com.example.counter.shared_types", output_root.join("java"))
//!        .expect("java type gen failed");
//!
//!    gen.typescript("shared_types", output_root.join("typescript"))
//!        .expect("typescript type gen failed");
//!}
//! ```

use serde::Deserialize;
use serde_generate::{java, swift, typescript, Encoding, SourceInstaller};
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
};
use thiserror::Error;

// Expose from `serde_reflection` for `register_type_with_samples()`
use serde_reflection::Samples;

use crate::App;

pub type Result = std::result::Result<(), TypeGenError>;

static DESERIALIZATION_ERROR_HINT: &str = r#"
This might be because you attempted to pass types with custom serialization across the FFI boundary. Make sure that:
1. Types you use in Event, ViewModel and Capabilities serialize as a container, otherwise wrap them in a new type struct,
    e.g. MyUuid(uuid::Uuid)
2. Sample values of such types have been provided to the type generator using TypeGen::register_samples, before any type registration."#;

#[derive(Error, Debug)]
pub enum TypeGenError {
    #[error("type tracing failed {0}")]
    TypeTracing(String),
    #[error("value tracing failed {0}")]
    ValueTracing(String),
    #[error("type tracing failed: {0} {}", DESERIALIZATION_ERROR_HINT)]
    Deserialization(String),
    #[error("code has been generated, too late to register types")]
    LateRegistration,
    #[error("type generation failed: {0}")]
    Generation(String),
    #[error("error writing generated types")]
    Io(#[from] std::io::Error),
}

#[derive(Debug)]
pub enum State {
    Registering(Tracer, Samples),
    Generating(Registry),
}

pub trait Export {
    fn register_types(generator: &mut TypeGen) -> Result;
}

/// The `TypeGen` struct stores the registered types so that they can be generated for foreign languages
/// use `TypeGen::new()` to create an instance
pub struct TypeGen {
    pub state: State,
}

impl Default for TypeGen {
    fn default() -> Self {
        TypeGen {
            state: State::Registering(Tracer::new(TracerConfig::default()), Samples::new()),
        }
    }
}

impl TypeGen {
    /// Creates an instance of the `TypeGen` struct
    pub fn new() -> Self {
        Default::default()
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling TypeGen::swift, TypeGen::java or TypeGen::typescript.
    /// This method would normally be called in a build.rs file of a sister crate responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the section on
    /// [creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    pub fn register_app<A: App>(&mut self) -> Result
    where
        A::Capabilities: Export,
        A::Event: Deserialize<'static>,
        A::ViewModel: Deserialize<'static> + 'static,
    {
        self.register_type::<A::Event>()?;
        self.register_type::<A::ViewModel>()?;

        A::Capabilities::register_types(self)?;

        Ok(())
    }

    /// Register sample values for types with custom serialization. This is necessary
    /// because the type registration relies on Serde to understand the structure of the types,
    /// and as part of the process runs a faux deserialization on each of them, with a best
    /// guess of a default value. If that default value does not deserialize, the type registration
    /// will fail.
    /// You can prevent this problem by registering a valid sample value (or values),
    /// which the deserialization will use instead.
    pub fn register_samples<'de, T>(&mut self, sample_data: Vec<T>) -> Result
    where
        T: serde::Deserialize<'de> + serde::Serialize,
    {
        match &mut self.state {
            State::Registering(tracer, samples) => {
                for sample in &sample_data {
                    match tracer.trace_value::<T>(samples, sample) {
                        Ok(_) => {}
                        Err(e) => return Err(TypeGenError::ValueTracing(e.explanation())),
                    }
                }
                Ok(())
            }
            _ => Err(TypeGenError::LateRegistration),
        }
    }
    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use serde::{Serialize, Deserialize};
    /// # use anyhow::Error;
    /// #[derive(Serialize, Deserialize)]
    /// enum MyNestedEnum { None }
    /// #[derive(Serialize, Deserialize)]
    /// enum MyEnum { None, Nested(MyNestedEnum) }
    /// fn register() -> Result<(), Error> {
    ///   let mut gen = TypeGen::new();
    ///   gen.register_type::<MyEnum>()?;
    ///   gen.register_type::<MyNestedEnum>()?;
    ///   Ok(())
    /// }
    /// ```
    pub fn register_type<'de, T>(&mut self) -> Result
    where
        T: serde::Deserialize<'de>,
    {
        match &mut self.state {
            State::Registering(tracer, _) => match tracer.trace_simple_type::<T>() {
                Ok(_) => Ok(()),
                Err(e @ serde_reflection::Error::DeserializationError(_)) => {
                    Err(TypeGenError::Deserialization(e.explanation()))
                }
                Err(e) => Err(TypeGenError::TypeTracing(e.explanation())),
            },
            _ => Err(TypeGenError::LateRegistration),
        }
    }

    /// Usually, the simple `register_type()` method can generate the types you need.
    /// Sometimes, though, you need to provide samples of your type. The `Uuid` type,
    /// for example, requires a sample struct to help the typegen system understand
    /// what it looks like. Use this method to provide samples when you register a
    /// type.
    ///
    /// For each of the types that you want to share with the Shell, call this method,
    /// providing samples of the type:
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use uuid::Uuid;
    /// # use serde::{Serialize, Deserialize};
    /// # use anyhow::Error;
    /// # #[derive(Serialize, Deserialize, Debug)]
    /// # struct MyUuid(Uuid);
    /// # fn register() -> Result<(), Error> {
    /// # let mut gen = TypeGen::new();
    ///   let sample_data = vec![MyUuid(Uuid::new_v4())];
    ///   gen.register_type_with_samples::<MyUuid>(sample_data)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note: Because of the way that enums are handled by `serde_reflection`,
    /// you may need to ensure that enums provided as samples have a first variant
    /// that does not use custom deserialization.
    pub fn register_type_with_samples<'de, T>(&'de mut self, sample_data: Vec<T>) -> Result
    where
        T: serde::Deserialize<'de> + serde::Serialize,
    {
        match &mut self.state {
            State::Registering(tracer, samples) => {
                for sample in &sample_data {
                    match tracer.trace_value::<T>(samples, sample) {
                        Ok(_) => {}
                        Err(e @ serde_reflection::Error::DeserializationError(_)) => {
                            return Err(TypeGenError::ValueTracing(e.explanation()))
                        }
                        Err(e) => return Err(TypeGenError::ValueTracing(e.explanation())),
                    }
                }

                match tracer.trace_type::<T>(samples) {
                    Ok(_) => Ok(()),
                    Err(e @ serde_reflection::Error::DeserializationError(_)) => {
                        Err(TypeGenError::Deserialization(e.explanation()))
                    }
                    Err(e) => Err(TypeGenError::TypeTracing(e.explanation())),
                }
            }
            _ => Err(TypeGenError::LateRegistration),
        }
    }

    /// Generates types for Swift
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut gen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// gen.swift("SharedTypes", output_root.join("swift"))
    ///     .expect("swift type gen failed");
    /// ```
    pub fn swift(&mut self, module_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry()?;

        let path = path.as_ref().join(module_name);

        fs::create_dir_all(&path)?;

        let installer = swift::Installer::new(path.clone());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

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
        write!(
            output,
            "{}",
            include_str!("../typegen_extensions/swift/requests.swift")
        )?;

        // wrap it all up in a swift package
        let mut output = File::create(path.join("Package.swift"))?;
        write!(
            output,
            "{}",
            include_str!("../typegen_extensions/swift/Package.swift")
                .replace("SharedTypes", module_name)
        )?;

        Ok(())
    }

    /// Generates types for Java (for use with Kotlin)
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut gen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// gen.java(
    ///     "com.redbadger.crux_core.shared_types",
    ///     output_root.join("java"),
    /// )
    /// .expect("java type gen failed");
    /// ```
    pub fn java(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;

        let config = serde_generate::CodeGeneratorConfig::new(package_name.to_string())
            .with_encodings(vec![Encoding::Bincode]);

        let installer = java::Installer::new(path.as_ref().to_path_buf());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        installer
            .install_module(&config, registry)
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let package_path = package_name.replace('.', "/");

        let requests = format!(
            "package {package_name};\n\n{}",
            include_str!("../typegen_extensions/java/Requests.java")
        );

        fs::write(
            path.as_ref()
                .to_path_buf()
                .join(package_path)
                .join("Requests.java"),
            requests,
        )?;

        Ok(())
    }

    /// Generates types for TypeScript
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut gen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// gen.typescript("shared_types", output_root.join("typescript"))
    ///    .expect("typescript type gen failed");
    /// ```
    pub fn typescript(&mut self, module_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;
        let output_dir = path.as_ref().to_path_buf();

        let installer = typescript::Installer::new(output_dir.clone());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let extensions_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("typegen_extensions/typescript");
        copy(extensions_dir, path).expect("Could not copy TS runtime");

        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

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

        let types_dir = output_dir.join("types");
        fs::create_dir_all(&types_dir)?;

        let mut output = File::create(types_dir.join(format!("{module_name}.ts")))?;
        write!(output, "{out}")?;

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

        Ok(())
    }

    fn ensure_registry(&mut self) -> Result {
        if let State::Registering(_, _) = self.state {
            // replace the current state with a dummy tracer
            let old_state = mem::replace(
                &mut self.state,
                State::Registering(Tracer::new(TracerConfig::default()), Samples::new()),
            );

            // convert tracer to registry
            if let State::Registering(tracer, _) = old_state {
                // replace dummy with registry
                self.state = State::Generating(
                    tracer
                        .registry()
                        .map_err(|e| TypeGenError::Generation(e.explanation()))?,
                );
            }
        }
        Ok(())
    }
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result {
    fs::create_dir_all(to.as_ref())?;

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

#[cfg(feature = "typegen")]
#[cfg(test)]
mod tests {
    use crate::typegen::TypeGen;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Serialize, Deserialize, Debug)]
    struct MyUuid(Uuid);

    #[test]
    fn test_typegen_for_uuid_without_samples() {
        let mut gen = TypeGen::new();
        let result = gen.register_type::<MyUuid>();

        assert!(
            result.is_err(),
            "typegen unexpectedly succeeded for Uuid, without samples"
        )
    }

    #[test]
    fn test_typegen_for_uuid_with_samples() {
        let sample_data = vec![MyUuid(Uuid::new_v4())];
        let mut gen = TypeGen::new();
        let result = gen.register_type_with_samples(sample_data);
        dbg!(&result);
        assert!(result.is_ok(), "typegen failed for Uuid, with samples");

        let sample_data = vec!["a".to_string(), "b".to_string()];
        let result = gen.register_type_with_samples(sample_data);
        assert!(result.is_ok(), "typegen failed with second sample data set");
    }
}
