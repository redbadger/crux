//! Generation of foreign language types (currently Swift, Kotlin, C#, TypeScript) for Crux
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
//! #         type Effect = Effect;
//! #         fn update(&self, _event: Event, _model: &mut Self::Model) -> Command<Effect, Event> {
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
//!  let typegen = TypeRegistry::new().register_app::<App>()?.build()?;
//!
//!  typegen.swift(
//!      &Config::builder("SharedTypes", &output_root.join("swift"))
//!      .add_extensions()
//!      .build()
//!  )?;
//!
//!  typegen.java(
//!      &Config::builder("com.crux.example.counter.shared", output_root.join("java"))
//!      .add_extensions()
//!      .build()
//!  )?;
//!
//!  typegen.csharp(
//!      &Config::builder("CounterApp.Shared", output_root.join("csharp"))
//!      .add_extensions()
//!      .build()
//!  )?;
//!
//!  typegen.typescript(
//!      &Config::builder("shared_types", output_root.join("typescript"))
//!      .add_extensions()
//!      .build()
//!  )?;
//! # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
//! ```
use std::{
    fs::{self, File},
    io::Write,
    process::Command,
    result::Result,
};

use facet::Facet;
pub use facet_generate::generation::{Config, ExternalPackage, PackageLocation};
use facet_generate::{
    Registry,
    generation::{
        Encoding, csharp, java, kotlin, swift,
        typescript::{self, InstallTarget},
    },
    reflection::RegistryBuilder,
};
use log::info;
use serde_json::json;
use thiserror::Error;

use crate::App;

macro_rules! extension_data {
    ($path:literal) => {
        include_str!(concat!("../../typegen_extensions/", $path))
    };
}

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

impl From<facet_generate::generation::Error> for TypeGenError {
    fn from(e: facet_generate::generation::Error) -> Self {
        TypeGenError::Generation(e.to_string())
    }
}

pub trait Export {
    /// Register types with the type registry.
    /// # Errors
    /// Returns a [`TypeGenError`] if the type generation fails.
    fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>;
}

impl Export for () {
    fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        Ok(registry)
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
    /// # Errors
    /// Returns a [`TypeGenError`] if the type registration fails.
    pub fn register_app<'a, A: App>(&mut self) -> Result<&mut Self, TypeGenError>
    where
        A::Effect: Export,
        A::Event: Facet<'a>,
        A::ViewModel: Facet<'a> + 'static,
    {
        A::Effect::register_types(self).map_err(|e| TypeGenError::Generation(e.to_string()))?;

        self.register_type::<A::Event>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?
            .register_type::<A::ViewModel>()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        Ok(self)
    }

    /// For each of the types that you want to share with the Shell, call this method:
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{TypeGenError, TypeRegistry};
    /// #[derive(facet::Facet)]
    /// struct MyStruct;
    ///
    /// #[derive(facet::Facet)]
    /// #[repr(C)]
    /// enum MyEnum { None }
    ///
    /// fn register() -> Result<(), TypeGenError> {
    ///   TypeRegistry::new()
    ///     .register_type::<MyEnum>()?
    ///     .register_type::<MyStruct>()?
    ///     .build()?;
    ///   Ok(())
    /// }
    /// ```
    /// # Errors
    /// Returns a [`TypeGenError`] if the type registration fails.
    pub fn register_type<'a, 'de, T>(&mut self) -> Result<&mut Self, TypeGenError>
    where
        T: Facet<'a>,
    {
        let builder = std::mem::take(&mut self.0);
        self.0 = builder.add_type::<T>().map_err(|e| {
            TypeGenError::Generation(format!(
                "couldn't register type {}: {e} {}",
                std::any::type_name::<T>(),
                T::SHAPE.type_identifier
            ))
        })?;

        Ok(self)
    }

    /// Builds the type registry and returns a [`CodeGenerator`] instance.
    /// # Errors
    /// Returns a [`TypeGenError`] if the type registration fails.
    pub fn build(&mut self) -> Result<CodeGenerator, TypeGenError> {
        let builder = std::mem::take(&mut self.0);
        let generator = CodeGenerator(
            builder
                .build()
                .map_err(|e| TypeGenError::Generation(e.to_string()))?,
        );

        Ok(generator)
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
    /// # let mut typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.swift(
    ///     &Config::builder("SharedTypes", output_root.join("swift"))
    ///     .add_extensions()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn swift(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Swift types");
        let path = config.out_dir.join(&config.package_name);
        let sources = path.join("Sources");

        fs::create_dir_all(&path)?;

        swift::Installer::new(&config.package_name, &path)
            .encoding(Encoding::Bincode)
            .external_packages(&config.external_packages)
            .generate(&self.0)?;

        if config.add_extensions {
            // add bincode deserialization for Vec<Request>
            let output_dir = sources.join(&config.package_name);
            fs::create_dir_all(&output_dir)?;
            let mut output = File::create(output_dir.join("Requests.swift"))?;

            let requests_data = extension_data!("swift/requests.swift");
            write!(output, "{requests_data}")?;
        }

        Ok(())
    }

    /// Generates types for Java
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.java(
    ///     &Config::builder("com.crux.example", output_root.join("java"))
    ///     .add_extensions()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    #[deprecated(
        since = "0.17.0",
        note = "The Java typegen is deprecated. Use Kotlin typegen instead."
    )]
    pub fn java(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Java types");
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        #[allow(deprecated)]
        java::Installer::new(&config.package_name, &config.out_dir)
            .encoding(Encoding::Bincode)
            .external_packages(&config.external_packages)
            .generate(&self.0)?;

        if config.add_extensions {
            let requests_data = extension_data!("java/Requests.java");
            let requests = format!("package {};\n\n{requests_data}", config.package_name);

            let output_dir = config.out_dir.join(package_path);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("Requests.java"), requests)?;
        }

        Ok(())
    }

    /// Generates types for Kotlin
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.kotlin(
    ///     &Config::builder("com.crux.example", output_root.join("kotlin"))
    ///     .add_extensions()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn kotlin(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Kotlin types");
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        kotlin::Installer::new(&config.package_name, &config.out_dir)
            .encoding(Encoding::Bincode)
            .external_packages(&config.external_packages)
            .generate(&self.0)?;

        if config.add_extensions {
            let requests_data = extension_data!("kotlin/Requests.kt");
            let requests = format!("package {};\n\n{requests_data}", config.package_name);

            let output_dir = config.out_dir.join(package_path);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("Requests.kt"), requests)?;
        }

        Ok(())
    }

    /// Generates types for C#
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.csharp(
    ///     &Config::builder("CounterApp.Shared", output_root.join("csharp"))
    ///     .add_extensions()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn csharp(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating C# types");
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        csharp::Installer::new(&config.package_name, &config.out_dir)
            .encoding(Encoding::Bincode)
            .external_packages(&config.external_packages)
            .generate(&self.0)?;

        if config.add_extensions {
            let requests_data = extension_data!("csharp/Requests.cs");
            let requests = format!("namespace {}{requests_data}", config.package_name);

            let output_dir = config.out_dir.join(package_path);
            fs::create_dir_all(&output_dir)?;
            fs::write(output_dir.join("Requests.cs"), requests)?;
        }

        Ok(())
    }

    /// Generates types for TypeScript
    /// e.g.
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let mut typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.typescript(
    ///     &Config::builder("shared_types", output_root.join("typescript"))
    ///     .add_extensions()
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn typescript(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating TypeScript types");
        fs::create_dir_all(&config.out_dir)?;
        let output_dir = &config.out_dir;

        typescript::Installer::new(&config.package_name, output_dir, InstallTarget::Node)
            .encoding(Encoding::Bincode)
            .external_packages(&config.external_packages)
            .generate(&self.0)?;

        let ts_config_str = serde_json::to_string_pretty(&json!({
            "compilerOptions": {
                "target": "es2020",
                "module": "commonjs",
                "declaration": true,
                "esModuleInterop": true,
                "strict": true,
                "esModuleInterop": true,
                "skipLibCheck": true,
                "forceConsistentCasingInFileNames": true
            }
        }))
        .map_err(|e| TypeGenError::Generation(e.to_string()))?;
        let mut output = File::create(output_dir.join("tsconfig.json"))?;
        write!(output, "{ts_config_str}")?;

        info!("Installing dependencies");
        Command::new("pnpm")
            .current_dir(output_dir)
            .arg("install")
            .status()
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => TypeGenError::PnpmNotFound(e),
                _ => TypeGenError::Io(e),
            })?;

        info!("Building TS code and emitting declarations");
        Command::new("pnpm")
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
}
