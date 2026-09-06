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
//! crux_core = { version = "0.20", features = ["facet_typegen"] }
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
//! use crux_core::type_generation::facet::{Config, TypeRegistry};
//! use tempfile::tempdir;
//! use shared::App;
//!
//! # fn main() -> Result<(), crux_core::type_generation::facet::TypeGenError> {
//! let tmp_dir = tempdir()?;
//! let output_root = tmp_dir.path();
//!
//! let typegen = TypeRegistry::new().register_app::<App>()?.build()?;
//!
//! typegen.swift(
//!     &Config::builder("SharedTypes", &output_root.join("swift"))
//!     .build()
//! )?;
//!
//! typegen.kotlin(
//!     &Config::builder("com.crux.example.counter.shared", output_root.join("kotlin"))
//!     .build()
//! )?;
//!
//! typegen.csharp(
//!     &Config::builder("CounterApp.Shared", output_root.join("csharp"))
//!     .build()
//! )?;
//!
//! typegen.typescript(
//!     &Config::builder("shared_types", output_root.join("typescript"))
//!     .build()
//! )?;
//! # Ok(())
//! # }
//! ```
mod effects;
mod plugins;

use std::{
    fs::{self, File},
    io::Write,
    process::Command,
    result::Result,
    sync::Arc,
};

use facet::Facet;
pub use facet_generate::generation::{Config, ExternalPackage, PackageLocation};
/// The shape of a type in the registry, as reflected by facet-generate. Part
/// of [`EffectVariantMeta`].
pub use facet_generate::reflection::format::{Format, QualifiedTypeName};
use facet_generate::{
    Registry,
    generation::{bincode::BincodePlugin, csharp, kotlin, swift, typescript},
    reflection::RegistryBuilder,
};
use log::info;
use serde_json::json;
use thiserror::Error;

pub use self::effects::{EffectBuilder, EffectMeta, EffectVariantMeta};
use self::plugins::{EffectHandlerPlugin, RequestKindPlugin};
use crate::App;

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
        Self::Generation(e.to_string())
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

/// Names the generated effect handler API claims in the root namespace of
/// every generated package. A registered type using one of these would be
/// silently shadowed, so [`TypeRegistry::build`] rejects it instead.
const RESERVED_TYPE_NAMES: &[&str] = &[
    "RequestKind",
    "EffectHandler",
    "IEffectHandler",
    "EffectSink",
    "IEffectSink",
    "EffectDispatcher",
];

pub struct TypeRegistry {
    builder: RegistryBuilder,
    effects: Vec<EffectMeta>,
}

pub struct CodeGenerator {
    registry: Registry,
    effects: Arc<[EffectMeta]>,
    handlers: bool,
}

/// The `TypeRegistry` struct stores the registered types so that they can be generated for foreign languages
/// use `TypeRegistry::new()` to create an instance
impl TypeRegistry {
    /// Creates an instance of the `TypeRegistry` struct for registration only
    #[must_use]
    pub fn new() -> Self {
        Self {
            builder: RegistryBuilder::new(),
            effects: Vec::new(),
        }
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling [`CodeGenerator::swift`] or [`CodeGenerator::typescript`].
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
        let builder = std::mem::take(&mut self.builder);
        self.builder = builder.add_type::<T>().map_err(|e| {
            TypeGenError::Generation(format!(
                "couldn't register type {}: {e} {}",
                std::any::type_name::<T>(),
                T::SHAPE.type_identifier
            ))
        })?;

        Ok(self)
    }

    /// Starts recording what type generation needs to know about the effect
    /// enum `E`: the [`RequestKind`](crate::RequestKind) each variant declares
    /// and the type its request resolves with.
    ///
    /// Called by `#[effect(facet_typegen)]`; you should not need to call it
    /// yourself. Register `E` with [`register_type`](Self::register_type)
    /// first, so that any `#[facet(rename)]` on it is already known.
    ///
    /// ```rust,ignore
    /// generator
    ///     .register_effect::<EffectFfi>()?
    ///     .variant::<RenderOperation>("Render")?
    ///     .variant::<HttpRequest>("Http")?
    ///     .finish();
    /// ```
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if `E` is not a named type.
    pub fn register_effect<'a, E: Facet<'a>>(&mut self) -> Result<EffectBuilder<'_>, TypeGenError> {
        let format = self.builder.format_of::<E>().map_err(|e| {
            TypeGenError::Generation(format!(
                "couldn't reflect effect {}: {e}",
                std::any::type_name::<E>()
            ))
        })?;

        let Format::TypeName(name) = format else {
            return Err(TypeGenError::Generation(format!(
                "effect {} is not a named type",
                std::any::type_name::<E>()
            )));
        };

        Ok(EffectBuilder::new(self, name))
    }

    /// Builds the type registry and returns a [`CodeGenerator`] instance.
    /// # Errors
    /// Returns a [`TypeGenError`] if the type registration fails, or if a
    /// registered type or effect variant claims one of the names the generated
    /// effect handler API uses.
    pub fn build(&mut self) -> Result<CodeGenerator, TypeGenError> {
        let builder = std::mem::take(&mut self.builder);
        let effects: Arc<[EffectMeta]> = std::mem::take(&mut self.effects).into();
        let registry = builder
            .build()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        if !effects.is_empty() {
            validate_names(&registry, &effects)?;
        }

        Ok(CodeGenerator {
            registry,
            effects,
            handlers: true,
        })
    }
}

/// Rejects registered types and effect variants that would collide with the
/// generated effect handler API.
fn validate_names(registry: &Registry, effects: &[EffectMeta]) -> Result<(), TypeGenError> {
    use facet_generate::reflection::format::Namespace;

    for name in registry.keys() {
        if name.namespace == Namespace::Root && RESERVED_TYPE_NAMES.contains(&name.name.as_str()) {
            return Err(TypeGenError::Generation(format!(
                "`{}` is generated for the effect handler API, so a shared type cannot be called that. Rename the type with `#[facet(rename = \"...\")]`.",
                name.name
            )));
        }
    }

    for effect in effects {
        for variant in &effect.variants {
            if variant.ident == "RequestKind" {
                return Err(TypeGenError::Generation(format!(
                    "effect `{}` has a variant called `RequestKind`, which collides with the generated request kind accessor. Rename the variant.",
                    effect.effect.name
                )));
            }
        }
    }

    Ok(())
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

        fs::create_dir_all(&path)?;

        let mut installer =
            swift::Installer::new(&config.package_name, &path).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(RequestKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects));
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

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

        let mut installer =
            kotlin::Installer::new(&config.package_name, &config.out_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(RequestKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects));
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

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

        let mut installer =
            csharp::Installer::new(&config.package_name, &config.out_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(RequestKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects));
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

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

        let mut installer =
            typescript::Installer::new(&config.package_name, output_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(RequestKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects));
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

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
        self.registry
    }

    /// What was recorded about each registered effect enum, in registration
    /// order.
    #[must_use]
    pub fn effects(&self) -> &[EffectMeta] {
        &self.effects
    }

    /// Turns off emission of the `RequestKind` type and the effect handler API.
    ///
    /// Only the types you registered are generated, exactly as before Crux
    /// 0.21. Use this if your shell dispatches effects by hand and the extra
    /// declarations are in the way.
    #[must_use]
    pub const fn without_effect_handlers(mut self) -> Self {
        self.handlers = false;
        self
    }
}
