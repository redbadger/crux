//! Generation of foreign language types (currently Swift, Java, TypeScript) for Crux
//!
//! To use this module, you can add a separate crate from your shared library, possibly
//! called `shared_types`, which will allow you to reference types from your shared library
//! during the build process (e.g. in `shared_types/build.rs`).
//!
//! This module is behind the feature called `facet_typegen`, and is not compiled into the default crate.
//!
//! Ensure that you have the following line in the `Cargo.toml` of your `shared_types` library.
//!
//! ```rust,ignore
//! [build-dependencies]
//! crux_core = { version = "0.15", features = ["facet_typegen"] }
//! ```
//!
//! * Your `shared_types` library, will have an empty `lib.rs`, since we only use it for generating foreign language type declarations.
//! * Create a `build.rs` in your `shared_types` library, that looks something like this:
//!
//! ```rust
//! # mod shared {
//! #     use crux_core::Command;
//! #     use crux_core::render::Render;
//! #     use crux_core::macros::Effect;
//! #     use facet::Facet;
//! #     use serde::{Deserialize, Serialize};
//! #     #[derive(Default)]
//! #     pub struct App;
//! #     #[derive(Facet, Serialize, Deserialize)]
//! #     #[repr(C)]
//! #     pub enum Event {
//! #         None,
//! #     }
//! #     #[derive(Facet, Serialize, Deserialize)]
//! #     pub struct ViewModel;
//! #     impl crux_core::App for App {
//! #         type Event = Event;
//! #         type Model = ();
//! #         type ViewModel = ViewModel;
//! #         type Capabilities = Capabilities;
//! #         type Effect = Effect;
//! #         fn update(&self, _event: Event, _model: &mut Self::Model, _caps: &Capabilities) -> Command<Effect, Event> {
//! #             todo!()
//! #         }
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
//!use crux_core::{bridge::Request, type_generation::facet::TypeGen};
//!use uuid::Uuid;
//!
//!#[test]
//!fn generate_types() -> anyhow::Result<()> {
//!    let mut typegen = TypeGen::new();
//!
//!    typegen.register_app::<App>()?;
//!
//!    let temp = assert_fs::TempDir::new()?;
//!    let output_root = temp.join("crux_core_typegen_test");
//!
//!    typegen.swift("SharedTypes", output_root.join("swift"))?;
//!
//!    typegen.java("com.example.counter.shared_types", output_root.join("java"))?;
//!
//!    typegen.typescript("shared_types", output_root.join("typescript"))?;
//!}
//! ```
use facet::Facet;
pub use facet_generate::generation::{Config, ExternalPackage, PackageLocation};
use facet_generate::{
    Registry,
    generation::{
        Encoding, SourceInstaller, java,
        module::{self, Module},
        swift, typescript,
    },
    reflection::RegistryBuilder,
};
use serde::Deserialize;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::App;

pub type Result = std::result::Result<(), TypeGenError>;

#[derive(Error, Debug)]
pub enum TypeGenError {
    #[error("code has been generated, too late to register types")]
    LateRegistration,
    #[error("type generation failed: {0}")]
    Generation(String),
    #[error("error writing generated types")]
    Io(#[from] std::io::Error),
    #[error(
        "`pnpm` is needed for TypeScript type generation, but it could not be found in PATH.\nPlease install it from https://pnpm.io/installation"
    )]
    PnpmNotFound(#[source] std::io::Error),
}

#[derive(Debug, Default)]
pub enum State {
    #[default]
    None,
    Registering(RegistryBuilder),
    Generating(Registry),
}

impl State {
    #[must_use]
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

pub trait Export {
    /// Register types with the type generator.
    ///
    /// This method should be called before any types are registered.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if the type tracing fails.
    fn register_types(generator: &mut TypeGen) -> Result;
}

impl Export for () {
    fn register_types(_generator: &mut TypeGen) -> Result {
        Ok(())
    }
}

/// The `TypeGen` struct stores the registered types so that they can be generated for foreign languages
/// use `TypeGen::new()` to create an instance
pub struct TypeGen {
    pub state: State,
}

impl Default for TypeGen {
    fn default() -> Self {
        TypeGen {
            state: State::Registering(RegistryBuilder::default()),
        }
    }
}

impl TypeGen {
    /// Creates an instance of the `TypeGen` struct
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling `TypeGen::swift`, `TypeGen::java` or `TypeGen::typescript`.
    /// This method would normally be called in a build.rs file of a sister crate responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the section on
    /// [creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    ///
    /// # Errors
    ///
    /// If any of the types used in the app cannot be registered, an error will be returned.
    pub fn register_app<'a, A: App>(&mut self) -> Result
    where
        A::Effect: Export,
        A::Event: Deserialize<'static> + Facet<'a>,
        A::ViewModel: Deserialize<'static> + Facet<'a> + 'static,
    {
        A::Effect::register_types(self)?;
        self.register_type::<A::Event>()?;
        self.register_type::<A::ViewModel>()?;

        Ok(())
    }

    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use serde::{Serialize, Deserialize};
    /// # use anyhow::Error;
    /// #[derive(facet::Facet, Serialize, Deserialize)]
    /// #[repr(C)]
    /// enum MyNestedEnum { None }
    /// #[derive(facet::Facet, Serialize, Deserialize)]
    /// #[repr(C)]
    /// enum MyEnum { None, Nested(MyNestedEnum) }
    /// fn register() -> Result<(), Error> {
    ///   let mut typegen = TypeGen::new();
    ///   typegen.register_type::<MyEnum>()?;
    ///   typegen.register_type::<MyNestedEnum>()?;
    ///   Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Errors that can occur during type registration.
    pub fn register_type<'a, 'de, T>(&mut self) -> Result
    where
        T: serde::Deserialize<'de> + Facet<'a>,
    {
        let State::Registering(builder) = self.state.take() else {
            return Err(TypeGenError::LateRegistration);
        };

        self.state = State::Registering(builder.add_type::<T>());

        Ok(())
    }

    /// Generates types for Swift
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeGen};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.swift(
    ///     Config::builder("SharedTypes", output_root.join("swift"))
    ///     .add_extensions()
    ///     .add_runtimes()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn swift(&mut self, config: Config) -> Result {
        self.ensure_registry();

        let path = config.out_dir.join(&config.package_name);
        let sources = path.join("Sources");

        fs::create_dir_all(&path)?;

        let mut installer = swift::Installer::new(
            config.package_name.to_string(),
            path,
            config.external_packages,
        );

        if config.add_runtimes {
            installer
                .install_serde_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
            installer
                .install_bincode_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        let State::Generating(ref registry) = self.state else {
            panic!("registry creation failed");
        };

        for (module, registry) in module::split(&config.package_name, registry) {
            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            installer
                .install_module(&config, &registry)
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        if config.add_extensions {
            // add bincode deserialization for Vec<Request>
            let output_dir = sources.join(&config.package_name);
            fs::create_dir_all(&output_dir)?;
            let mut output = File::create(output_dir.join("Requests.swift"))?;

            let requests_path = Self::extensions_path("swift/requests.swift");
            let requests_data = fs::read_to_string(requests_path)?;
            write!(output, "{requests_data}")?;
        }

        installer
            .install_manifest(&config.package_name)
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(())
    }

    /// Generates types for Java (for use with Kotlin)
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.java("com.crux.example", output_root.join("java"))?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn java(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry();

        fs::create_dir_all(&path)?;

        let package_path = package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(path.as_ref().join(&package_path)).unwrap_or(());

        let mut installer = java::Installer::new(path.as_ref().to_path_buf());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let State::Generating(ref mut registry) = self.state else {
            panic!("registry creation failed");
        };

        for (module, registry) in module::split(package_name, registry) {
            let this_module = &module.config().module_name;
            let is_root_package = package_name == this_module;
            let module = if is_root_package {
                module
            } else {
                Module::new([package_name, this_module].join("."))
            };

            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            installer
                .install_module(&config, &registry)
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        let requests_path = Self::extensions_path("java/Requests.java");

        let requests_data = fs::read_to_string(requests_path)?;

        let requests = format!("package {package_name};\n\n{requests_data}");

        let output_dir = path.as_ref().join(package_path);
        fs::create_dir_all(&output_dir)?;
        fs::write(output_dir.join("Requests.java"), requests)?;

        Ok(())
    }

    /// Generates types for TypeScript
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeGen;
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeGen::new();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.typescript("shared_types", output_root.join("typescript"))?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    /// # Errors
    /// Errors that can occur during type generation.
    ///
    /// # Panics
    /// Panics if the registry creation fails.
    pub fn typescript(&mut self, package_name: &str, path: impl AsRef<Path>) -> Result {
        self.ensure_registry();

        fs::create_dir_all(&path)?;
        let output_dir = path.as_ref().to_path_buf();

        let types_dir = output_dir.join("types");
        fs::create_dir_all(&types_dir)?;

        let mut installer = typescript::Installer::new(output_dir.clone());
        installer
            .install_serde_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        installer
            .install_bincode_runtime()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        let State::Generating(ref mut registry) = self.state else {
            panic!("registry creation failed");
        };

        for (module, registry) in module::split(package_name, registry) {
            let config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            let module_name = config.module_name();

            let generator = typescript::CodeGenerator::new(&config);
            let mut source = Vec::new();
            generator.output(&mut source, &registry)?;

            // FIXME fix import paths in generated code which assume running on Deno
            let out = String::from_utf8_lossy(&source)
                .replace(
                    "import { BcsSerializer, BcsDeserializer } from '../bcs/mod.ts';",
                    "",
                )
                .replace(".ts'", "'");

            let mut output = File::create(types_dir.join(format!("{module_name}.ts")))?;
            write!(output, "{out}")?;
        }

        let extensions_dir = Self::extensions_path("typescript");
        copy(extensions_dir, path)?;

        // Install dependencies
        std::process::Command::new("pnpm")
            .current_dir(output_dir.clone())
            .arg("install")
            .status()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => TypeGenError::PnpmNotFound(e),
                _ => TypeGenError::Io(e),
            })?;

        // Build TS code and emit declarations
        std::process::Command::new("pnpm")
            .current_dir(output_dir)
            .arg("exec")
            .arg("tsc")
            .arg("--build")
            .status()
            .map_err(TypeGenError::Io)?;

        Ok(())
    }

    fn ensure_registry(&mut self) {
        if let State::Registering(_) = self.state {
            if let State::Registering(registry) = self.state.take() {
                self.state = State::Generating(registry.build());
            }
        }
    }

    fn extensions_path(path: &str) -> PathBuf {
        let custom = PathBuf::from("./typegen_extensions").join(path);
        let default = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("typegen_extensions")
            .join(path);

        match custom.try_exists() {
            Ok(true) => custom,
            Ok(false) => default,
            Err(e) => {
                println!("cant check typegen extensions override: {e}");
                default
            }
        }
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
        }
    }

    Ok(())
}
