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
//! crux_core = { version = "0.3", features = ["typegen"] }
//! ```
//!
//! * Your `shared_types` library, will have an empty `lib.rs`, since we only use it for generating foreign language type declarations.
//! * Create a `build.rs` in your `shared_types` library, that looks something like this:
//!
//! ```rust
//! # mod shared {
//! #     use crux_http::Http;
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
//! #         pub http: Http<Event>,
//! #     }
//! # }
//! use anyhow::Result;
//! use crux_core::{typegen::TypeGen, bridge::Request};
//! use crux_http::protocol::{HttpRequest, HttpResponse};
//! use shared::{EffectFfi, Event, ViewModel};
//! use std::path::PathBuf;
//!
//! fn main() {
//!     println!("cargo:rerun-if-changed=../shared");
//!
//!     let mut gen = TypeGen::new();
//!
//!     register_types(&mut gen).expect("type registration failed");
//!
//!     let output_root = std::env::temp_dir().join("crux_core_typegen_doctest");
//!
//!     gen.swift("shared_types", output_root.join("swift"))
//!         .expect("swift type gen failed");
//!
//!     gen.java(
//!         "com.redbadger.catfacts.shared_types",
//!         output_root.join("java"),
//!     )
//!     .expect("java type gen failed");
//!
//!     gen.typescript("shared_types", output_root.join("typescript"))
//!         .expect("typescript type gen failed");
//! }
//!
//! fn register_types(gen: &mut TypeGen) -> Result<()> {
//!     gen.register_type::<Request<EffectFfi>>()?;
//!
//!     gen.register_type::<EffectFfi>()?;
//!     gen.register_type::<HttpRequest>()?;
//!
//!     let sample_events = vec![Event::SendUuid(uuid::Uuid::new_v4())];
//!     gen.register_type_with_samples(sample_events)?;
//!
//!     gen.register_type::<HttpResponse>()?;
//!
//!     gen.register_type::<ViewModel>()?;
//!     Ok(())
//! }
//! ```

use anyhow::{anyhow, bail, Result};
use serde_generate::Encoding;
use serde_reflection::{Registry, Tracer, TracerConfig};
use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
};

// Expose from `serde_reflection` for `register_type_with_samples()`
use serde_reflection::Samples;

use crate::App;

#[derive(Debug)]
pub enum State {
    Registering(Tracer, Samples),
    Generating(Registry),
}

pub trait Export {
    fn register_types(generator: &mut TypeGen) -> Result<()>;
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

static DESERIALIZATION_ERROR_HINT: &str = r#"
This might be because you attempted to pass types with custom serialization across the FFI boundary. Make sure that:
1. Types you use in Event, ViewModel and Capabilities serialize as a container, otherwise wrap them in a new type struct,
    e.g. MyUuid(uuid::Uuid)
2. Sample values of such types have been provided to the type generator using TypeGen::register_samples, before any type registration."#;

impl TypeGen {
    /// Creates an instance of the `TypeGen` struct
    pub fn new() -> Self {
        Default::default()
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling TypeGen::swift, TypeGen::java or TypeGen::typescript.
    /// This method would normally be called in a build.rs file of a sister create responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the [section on creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    pub fn register_app<A: App>(&mut self) -> Result<()>
    where
        <A as App>::Capabilities: Export,
    {
        self.register_type::<A::Event>()?;
        self.register_type::<A::ViewModel>()?;

        <A::Capabilities as Export>::register_types(self)?;

        Ok(())
    }

    /// Register sample values for types with custom serialization. This is necessary
    /// because the type registration relies on Serde to understand the structure of the types,
    /// and as part of the process runs a faux deserialization on each of them, with a best
    /// guess of a default value. If that default value does not deserialize, the type registration
    /// will fail.
    /// You can prevent this problem by registering a valid sample value (or values) which the deserialization will use instead.
    pub fn register_samples<'de, T>(&'de mut self, sample_data: Vec<T>) -> Result<()>
    where
        T: serde::Deserialize<'de> + serde::Serialize + std::fmt::Debug,
    {
        match &mut self.state {
            State::Registering(tracer, samples) => {
                for sample in &sample_data {
                    match tracer.trace_value::<T>(samples, sample) {
                        Ok(_) => {}
                        Err(e) => bail!("value tracing failed: {}", e),
                    }
                }
            }
            _ => bail!("code has been generated, too late to register types"),
        }

        Ok(())
    }
    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use std::marker::PhantomData;
    /// # use serde::{Serialize, Deserialize};
    /// # use anyhow::Error;
    /// # #[derive(Serialize, Deserialize)]
    /// # struct Request<T> { phantom: PhantomData<T> }
    /// # #[derive(Serialize, Deserialize)]
    /// # struct Effect {}
    /// # #[derive(Serialize, Deserialize)]
    /// # struct Event {}
    /// # #[derive(Serialize, Deserialize)]
    /// # struct ViewModel {}
    /// # fn register() -> Result<(), Error> {
    /// # let mut gen = TypeGen::new();
    ///   gen.register_type::<Request<Effect>>()?;
    ///   gen.register_type::<Effect>()?;
    ///   gen.register_type::<Event>()?;
    ///   gen.register_type::<ViewModel>()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn register_type<'de, T>(&mut self) -> Result<()>
    where
        T: serde::Deserialize<'de>,
    {
        match &mut self.state {
            State::Registering(tracer, _) => match tracer.trace_simple_type::<T>() {
                Ok(_) => Ok(()),
                Err(serde_reflection::Error::DeserializationError(text)) => {
                    bail!(
                        "Type tracing failed: {}. {}",
                        text,
                        DESERIALIZATION_ERROR_HINT
                    )
                }
                Err(e) => bail!("type tracing failed: {}", e),
            },
            _ => bail!("code has been generated, too late to register types"),
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
    /// # #[derive(Serialize, Deserialize)]
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
    pub fn register_type_with_samples<'de, T>(&'de mut self, sample_data: Vec<T>) -> Result<()>
    where
        T: serde::Deserialize<'de> + serde::Serialize + std::fmt::Debug,
    {
        match &mut self.state {
            State::Registering(tracer, samples) => {
                for sample in &sample_data {
                    match tracer.trace_value::<T>(samples, sample) {
                        Ok(_) => {}
                        Err(serde_reflection::Error::DeserializationError(text)) => {
                            bail!(
                                "Type tracing failed: {}. {}",
                                text,
                                DESERIALIZATION_ERROR_HINT
                            )
                        }
                        Err(e) => bail!("value tracing failed: {}", e),
                    }
                }

                match tracer.trace_type::<T>(samples) {
                    Ok(_) => Ok(()),
                    Err(e) => bail!("Type tracing failed: {}. {}", e, DESERIALIZATION_ERROR_HINT),
                }
            }
            _ => bail!("code has been generated, too late to register types"),
        }
    }

    /// Generates types for Swift
    /// e.g.
    /// ```rust
    /// # use crux_core::typegen::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut gen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// gen.swift("shared_types", output_root.join("swift"))
    ///     .expect("swift type gen failed");
    /// ```
    pub fn swift(&mut self, module_name: &str, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;

        let mut source = Vec::new();
        let config = serde_generate::CodeGeneratorConfig::new("shared".to_string())
            .with_encodings(vec![Encoding::Bcs]);

        let generator = serde_generate::swift::CodeGenerator::new(&config);
        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        generator.output(&mut source, registry)?;

        // FIXME workaround for odd namespacing behaviour in Swift output
        // which as far as I can tell does not support namespaces in this way
        let out = String::from_utf8_lossy(&source).replace("shared.", "");

        let out = format!(
            "{out}\n\n{}",
            include_str!("../typegen_extensions/swift/requests.swift")
        );

        let path = path
            .as_ref()
            .to_path_buf()
            .join(format!("{module_name}.swift"));
        let mut output = File::create(path)?;
        write!(output, "{out}")?;

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
    pub fn java(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;

        let config = serde_generate::CodeGeneratorConfig::new(package_name.to_string())
            .with_encodings(vec![Encoding::Bcs]);

        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        let generator = serde_generate::java::CodeGenerator::new(&config);
        generator.write_source_files(path.as_ref().to_path_buf(), registry)?;

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
    pub fn typescript(&mut self, module_name: &str, path: impl AsRef<Path>) -> Result<()> {
        self.ensure_registry()?;

        fs::create_dir_all(&path)?;
        let output_dir = path.as_ref().to_path_buf();

        let extensions_dir =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("typegen_extensions/typescript");
        let mut source = Vec::new();

        // FIXME this should be the actual route, but the runtime is built
        // for Deno, so we patch it heavily in extensions:
        //
        // let installer = typescript::Installer::new(output_dir.clone());
        // installer.install_serde_runtime()?;
        // installer.install_bcs_runtime()?;
        copy(extensions_dir, path).expect("Could not copy TS runtime");

        let registry = match &self.state {
            State::Generating(registry) => registry,
            _ => panic!("registry creation failed"),
        };

        let config = serde_generate::CodeGeneratorConfig::new(module_name.to_string())
            .with_serialization(true)
            .with_encodings(vec![Encoding::Bcs]);

        let generator = serde_generate::typescript::CodeGenerator::new(&config);
        generator.output(&mut source, registry)?;
        // FIXME fix import paths in generated code which assume running on Deno
        let out = String::from_utf8_lossy(&source).replace(".ts'", "'");

        let types_dir = output_dir.join("types");
        fs::create_dir_all(types_dir)?;

        let mut output = File::create(output_dir.join(format!("types/{module_name}.ts")))?;
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

    fn ensure_registry(&mut self) -> Result<()> {
        if let State::Registering(_, _) = self.state {
            // replace the current state with a dummy tracer
            let old_state = mem::replace(
                &mut self.state,
                State::Registering(Tracer::new(TracerConfig::default()), Samples::new()),
            );

            // convert tracer to registry
            if let State::Registering(tracer, _) = old_state {
                // replace dummy with registry
                self.state = State::Generating(tracer.registry().map_err(|e| anyhow!("{e}"))?);
            }
        }
        Ok(())
    }
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<()> {
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
        assert!(result.is_ok(), "typegen failed for Uuid, with samples");

        let sample_data = vec!["a".to_string(), "b".to_string()];
        let result = gen.register_type_with_samples(sample_data);
        assert!(result.is_ok(), "typegen failed with second sample data set");
    }
}
