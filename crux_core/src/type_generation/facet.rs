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
//! #     use crux_core::render::RenderOperation;
//! #     use crux_core::macros::effect;
//! #     use facet::Facet;
//! #     #[derive(Default)]
//! #     pub struct App;
//! #     #[derive(Facet)]
//! #     #[repr(C)]
//! #     pub enum Event {
//! #         None,
//! #     }
//! #     #[effect(facet_typegen)]
//! #     pub enum Effect {
//! #         Render(RenderOperation),
//! #     }
//! #     #[derive(Facet)]
//! #     pub struct ViewModel;
//! #     impl crux_core::App for App {
//! #         type Event = Event;
//! #         type Model = ();
//! #         type ViewModel = ViewModel;
//! #         type Capabilities = ();
//! #         type Effect = Effect;
//! #         fn update(&self, _event: Event, _model: &mut Self::Model, _caps: &()) -> Command<Effect, Event> {
//! #             todo!()
//! #         }
//! #         fn view(&self, _model: &Self::Model) -> Self::ViewModel {
//! #             todo!();
//! #         }
//! #     }
//! # }
//! # use std::path::PathBuf;
//!use crux_core::type_generation::facet::{Config, TypeRegistry};
//!use tempfile::tempdir;
//!use shared::App;
//!
//!  let tmp_dir = tempdir()?;
//!  let output_root = tmp_dir.path();
//!
//!  let typegen = TypeRegistry::new().register_app::<App>().build();
//!
//!  typegen.swift(
//!      &Config::builder("SharedTypes", &output_root.join("swift"))
//!      .add_extensions()
//!      .add_runtimes()
//!      .build()
//!  )?;
//!
//!  typegen.java(
//!      &Config::builder("com.crux.example.counter.shared", output_root.join("java"))
//!      .add_extensions()
//!      .add_runtimes()
//!      .build()
//!  )?;
//!
//!  typegen.typescript(
//!      &Config::builder("shared_types", output_root.join("typescript"))
//!      .add_extensions()
//!      .add_runtimes()
//!      .build()
//!  )?;
//! # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
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
    #[error("type generation failed: {0}")]
    Generation(String),
    #[error("error writing generated types")]
    Io(#[from] std::io::Error),
    #[error(
        "`pnpm` is needed for TypeScript type generation, but it could not be found in PATH.\nPlease install it from https://pnpm.io/installation"
    )]
    PnpmNotFound(#[source] std::io::Error),
}

pub trait Export {
    /// Register types with the type registry.
    fn register_types(registry: &mut TypeRegistry) -> &mut TypeRegistry;
}

impl Export for () {
    fn register_types(registry: &mut TypeRegistry) -> &mut TypeRegistry {
        registry
    }
}

pub struct TypeRegistry(RegistryBuilder);

pub struct CodeGenerator(Registry);

/// The `TypeRegistry` struct stores the registered types so that they can be generated for foreign languages
/// use `TypeRegistry::new()` to create an instance
impl TypeRegistry {
    /// Creates an instance of the `TypeRegistry` struct for registration only
    #[must_use]
    pub fn new() -> Self {
        Self(RegistryBuilder::new())
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling [`CodeGenerator::swift`], [`CodeGenerator::java`] or [`CodeGenerator::typescript`].
    /// This method would normally be called in a build.rs file of a sister crate responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the section on
    /// [creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    pub fn register_app<'a, A: App>(&mut self) -> &mut Self
    where
        A::Effect: Export,
        A::Event: Facet<'a>,
        A::ViewModel: Facet<'a> + 'static,
    {
        A::Effect::register_types(self);
        self.register_type::<A::Event>()
            .register_type::<A::ViewModel>()
    }

    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::TypeRegistry;
    /// #[derive(facet::Facet)]
    /// struct MyStruct;
    ///
    /// #[derive(facet::Facet)]
    /// #[repr(C)]
    /// enum MyEnum { None }
    ///
    /// fn register() {
    ///   TypeRegistry::new()
    ///     .register_type::<MyEnum>()
    ///     .register_type::<MyStruct>()
    ///     .build();
    /// }
    /// ```
    pub fn register_type<'a, 'de, T>(&mut self) -> &mut Self
    where
        T: Facet<'a>,
    {
        let builder = std::mem::take(&mut self.0);
        self.0 = builder.add_type::<T>();

        self
    }

    #[must_use]
    pub fn build(&mut self) -> CodeGenerator {
        let builder = std::mem::take(&mut self.0);
        CodeGenerator(builder.build())
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator {
    /// Generates types for Swift
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.swift(
    ///     &Config::builder("SharedTypes", output_root.join("swift"))
    ///     .add_extensions()
    ///     .add_runtimes()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn swift(&self, config: &Config) -> Result {
        let path = config.out_dir.join(&config.package_name);
        let sources = path.join("Sources");

        fs::create_dir_all(&path)?;

        let mut installer =
            swift::Installer::new(&config.package_name, &path, &config.external_packages);

        if config.add_runtimes {
            installer
                .install_serde_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
            installer
                .install_bincode_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        for (module, registry) in module::split(&config.package_name, &self.0) {
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
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.java(
    ///     &Config::builder("com.crux.example", output_root.join("java"))
    ///     .add_extensions()
    ///     .add_runtimes()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn java(&self, config: &Config) -> Result {
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        let mut installer = java::Installer::new(&config.out_dir);
        if config.add_runtimes {
            installer
                .install_serde_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
            installer
                .install_bincode_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        for (module, registry) in module::split(&config.package_name, &self.0) {
            let this_module = &module.config().module_name;
            let is_root_package = config.package_name == *this_module;
            let module = if is_root_package {
                module
            } else {
                Module::new(format!("{}.{}", config.package_name, this_module))
            };

            let module_config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            installer
                .install_module(&module_config, &registry)
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        if config.add_extensions {
            let requests_path = Self::extensions_path("java/Requests.java");

            let requests_data = fs::read_to_string(requests_path)?;

            let requests = format!("package {};\n\n{requests_data}", config.package_name);

            let output_dir = config.out_dir.join(package_path);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("Requests.java"), requests)?;
        }

        Ok(())
    }

    /// Generates types for TypeScript
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build();
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.typescript(
    ///     &Config::builder("shared_types", output_root.join("typescript"))
    ///     .add_extensions()
    ///     .add_runtimes()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn typescript(&self, config: &Config) -> Result {
        fs::create_dir_all(&config.out_dir)?;
        let output_dir = &config.out_dir;

        let types_dir = output_dir.join("types");
        fs::create_dir_all(&types_dir)?;

        let mut installer = typescript::Installer::new(output_dir);
        if config.add_runtimes {
            installer
                .install_serde_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
            installer
                .install_bincode_runtime()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        }

        for (module, registry) in module::split(&config.package_name, &self.0) {
            let module_config = module
                .config()
                .clone()
                .with_encodings(vec![Encoding::Bincode]);

            let module_name = module_config.module_name();

            let generator = typescript::CodeGenerator::new(&module_config);
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

        if config.add_extensions {
            let extensions_dir = Self::extensions_path("typescript");
            copy(extensions_dir, &config.out_dir)?;
        }

        // Install dependencies
        std::process::Command::new("pnpm")
            .current_dir(output_dir)
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

    /// Consumes the generator and returns the registry
    #[must_use]
    pub fn registry(self) -> Registry {
        self.0
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
