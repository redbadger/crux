//! Generation of foreign language types (currently Swift, Kotlin, C#, TypeScript) for Crux
//!
//! Type generation runs from your core crate itself — the one usually called
//! `shared` — which carries a small binary to drive it. There is no separate
//! crate for it: the examples put the binary in `shared/src/bin/codegen.rs`
//! and gate it on a feature, so an ordinary build of the core never compiles
//! it.
//!
//! ```toml
//! [[bin]]
//! name = "codegen"
//! required-features = ["codegen"]
//!
//! [features]
//! facet_typegen = ["crux_core/facet_typegen"]
//! codegen = ["facet_typegen", "dep:anyhow", "dep:clap"]
//! ```
//!
//! This module is behind the `facet_typegen` feature, so it is never compiled
//! into the core you ship.
//!
//! The binary takes the language and the output directory from its arguments
//! and generates into them; stripped of that plumbing, it does this:
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
//!     &Config::builder("App", &output_root.join("swift"))
//!     .build()
//! )?;
//!
//! typegen.kotlin(
//!     &Config::builder("com.crux.examples.counter", output_root.join("kotlin"))
//!     .build()
//! )?;
//!
//! typegen.csharp(
//!     &Config::builder("CounterApp.Shared", output_root.join("csharp"))
//!     .build()
//! )?;
//!
//! typegen.typescript(
//!     &Config::builder("app", output_root.join("typescript"))
//!     .build()
//! )?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Bridging to the FFI bindings
//!
//! The generated `Core` talks to the core through a `CoreBridge`, which a
//! shell normally implements over its FFI bindings. Name the `BoltFFI` bindings
//! with [`CodeGenerator::boltffi`] and that adapter is generated too, as an
//! `FfiBridge`, along with a one-argument `Core` constructor that uses it:
//!
//! ```rust
//! # use crux_core::type_generation::facet::{BoltFfi, PackageLocation, TypeRegistry};
//! let typegen = TypeRegistry::new().build()?.boltffi(
//!     BoltFfi::new()
//!         .swift("Shared")
//!         .kotlin()
//!         .typescript("shared", PackageLocation::Path("../pkg".into()))
//!         .csharp(),
//! );
//! # let _ = typegen;
//! # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
//! ```
//!
//! The shell then writes `Core(handler: MyHandler())` in Swift,
//! `Core(handler, scope)` in Kotlin, `await Core.create(handler, onView)` in
//! TypeScript and `new Core(handler)` in C#. A language you do not name is
//! generated exactly as before.
//!
//! ## Shell handlers shipped with a capability
//!
//! A capability crate can ship the shell side of its protocol — the rules for
//! turning a platform response into its operation's output — as source. Name
//! the ones you want with [`TypeRegistry::shell_handler`] and they are
//! emitted into the generated module:
//!
//! ```rust
//! use crux_core::type_generation::facet::{
//!     ShellHandler, ShellSource, TypeGenError, TypeRegistry,
//! };
//!
//! // Normally `crux_http::HTTP`, declared by the capability crate, whose
//! // source is `include_str!` from the crate rather than written inline.
//! static HTTP: ShellHandler = ShellHandler::new("Http")
//!     .types(register_types)
//!     .swift(ShellSource::stdlib("public protocol HttpHandler: Sendable {}"));
//!
//! // A real capability registers each of its operations here, so that the
//! // types its sources name exist even when the app never sends them:
//! // `HttpRequest::register_types_facet(registry)`.
//! fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
//!     Ok(registry)
//! }
//!
//! let typegen = TypeRegistry::new().shell_handler(&HTTP)?.build()?;
//! # let _ = typegen;
//! # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
//! ```
//!
//! Registering the handler also registers the types its sources name, so the
//! whole capability is generated even when the app's `Effect` uses a subset of
//! it. Nothing generated calls the shipped implementation: your handler holds
//! an instance and delegates to it, one line per operation.
mod app;
mod boltffi;
mod effects;
mod plugins;
mod shell_handler;

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

pub use self::app::AppMeta;
pub use self::boltffi::{BoltFfi, CSharpFfi, KotlinFfi, SwiftFfi, TypeScriptFfi};
pub use self::effects::{EffectBuilder, EffectMeta, EffectVariantMeta};
use self::plugins::{CorePlugin, EffectHandlerPlugin, OperationKindPlugin, ShellHandlerPlugin};
pub use self::shell_handler::{RegisterTypes, ShellHandler, ShellSource};
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

/// Names the generated shell API claims in the root namespace of every
/// generated package. A registered type using one of these would be silently
/// shadowed, so [`TypeRegistry::build`] rejects it instead.
const RESERVED_TYPE_NAMES: &[&str] = &[
    "OperationKind",
    "EffectHandler",
    "IEffectHandler",
    "EffectSink",
    "IEffectSink",
    "EffectDispatcher",
    "Core",
    "CoreBridge",
    "ICoreBridge",
    // Only emitted when `boltffi` is configured, but reserved unconditionally:
    // whether a shared type clashes should not depend on how the shell is
    // wired up.
    "FfiBridge",
];

pub struct TypeRegistry {
    builder: RegistryBuilder,
    effects: Vec<EffectMeta>,
    app: Option<AppMeta>,
    shell_handlers: Vec<&'static ShellHandler>,
}

pub struct CodeGenerator {
    registry: Registry,
    effects: Arc<[EffectMeta]>,
    app: Option<AppMeta>,
    handlers: bool,
    core: bool,
    boltffi: Option<BoltFfi>,
    shell_handlers: Vec<&'static ShellHandler>,
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
            app: None,
            shell_handlers: Vec::new(),
        }
    }

    /// Registers the shell handler a capability ships, and the types its
    /// sources name.
    ///
    /// The source is the capability's — a `HttpHandler` protocol and an
    /// implementation of it, in each language the capability ships — and it is
    /// emitted verbatim, into `Http.swift`, `Http.kt` and `Http.cs` beside the
    /// module, and after the types in TypeScript. Nothing generated calls it:
    /// the shell's handler holds an instance and delegates to it, one line per
    /// operation.
    ///
    /// ```rust,ignore
    /// let typegen = TypeRegistry::new()
    ///     .register_app::<Weather>()?
    ///     .shell_handler(&crux_http::HTTP)?
    ///     .shell_handler(&crux_time::TIME)?
    ///     .build()?;
    /// ```
    ///
    /// A shipped source implements the whole capability, so registering the
    /// handler registers every type it names — including the operations this
    /// app never sends. The capability says which those are, with
    /// [`ShellHandler::types`].
    ///
    /// The capability declares the handler with [`ShellHandler::new`] and one
    /// builder per language it ships:
    ///
    /// ```rust,ignore
    /// pub static HTTP: ShellHandler = ShellHandler::new("Http")
    ///     .types(register_types)
    ///     .swift(ShellSource::stdlib(include_str!("../shell/swift/Http.swift")))
    ///     .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt")));
    /// ```
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if one of the handler's own types cannot be
    /// registered. Registering a handler twice, or one whose name collides
    /// with a type of yours, is reported by [`build`](Self::build).
    pub fn shell_handler(
        &mut self,
        handler: &'static ShellHandler,
    ) -> Result<&mut Self, TypeGenError> {
        self.shell_handlers.push(handler);

        if let Some(register) = handler.types {
            register(self)?;
        }

        Ok(self)
    }

    /// Register all the types used in app `A` to be shared with the Shell.
    ///
    /// Do this before calling [`CodeGenerator::swift`] or [`CodeGenerator::typescript`].
    /// This method would normally be called in a build.rs file of a sister crate responsible for
    /// creating "foreign language" type definitions for the FFI boundary.
    /// See the section on
    /// [creating the shared types crate](https://redbadger.github.io/crux/getting_started/core.html#create-the-shared-types-crate)
    /// in the Crux book for more information.
    /// The `Event` and `ViewModel` of the first app registered are also
    /// recorded as an [`AppMeta`], which is what the generated `Core` names in
    /// its signatures.
    ///
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

        // The generated `Core` uses fixed names, so — like the handler API,
        // which is emitted for the first registered effect — the first app
        // registered is the one it is generated for.
        if self.app.is_none() {
            let event = self.named_type::<A::Event>("event")?;
            let view_model = self.named_type::<A::ViewModel>("view model")?;
            self.app = Some(AppMeta { event, view_model });
        }

        Ok(self)
    }

    /// The registry name of `T`, for the metadata the plugins read.
    fn named_type<'a, T: Facet<'a>>(&self, what: &str) -> Result<QualifiedTypeName, TypeGenError> {
        let format = self.builder.format_of::<T>().map_err(|e| {
            TypeGenError::Generation(format!(
                "couldn't reflect {what} {}: {e}",
                std::any::type_name::<T>()
            ))
        })?;

        let Format::TypeName(name) = format else {
            return Err(TypeGenError::Generation(format!(
                "{what} {} is not a named type",
                std::any::type_name::<T>()
            )));
        };

        Ok(name)
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
    /// enum `E`: the [`OperationKind`](crate::OperationKind) each variant declares
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
    /// registered type, effect variant or shell handler claims one of the
    /// names the generated effect handler API uses.
    pub fn build(&mut self) -> Result<CodeGenerator, TypeGenError> {
        let builder = std::mem::take(&mut self.builder);
        let effects: Arc<[EffectMeta]> = std::mem::take(&mut self.effects).into();
        let shell_handlers = std::mem::take(&mut self.shell_handlers);
        let registry = builder
            .build()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        if !effects.is_empty() {
            validate_names(&registry, &effects)?;
        }
        validate_shell_handlers(&registry, &shell_handlers)?;

        Ok(CodeGenerator {
            registry,
            effects,
            app: self.app.clone(),
            handlers: true,
            core: true,
            boltffi: None,
            shell_handlers,
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
                "`{}` is generated for the shell API, so a shared type cannot be called that. Rename the type with `#[facet(rename = \"...\")]`.",
                name.name
            )));
        }
    }

    for effect in effects {
        for variant in &effect.variants {
            if variant.ident == "OperationKind" {
                return Err(TypeGenError::Generation(format!(
                    "effect `{}` has a variant called `OperationKind`, which collides with the generated operation kind accessor. Rename the variant.",
                    effect.effect.name
                )));
            }
        }
    }

    Ok(())
}

/// Rejects registered shell handlers whose names would collide — with each
/// other, with the generated shell API, or with a type the app registered.
///
/// The names are checked whatever language is generated and whether or not
/// that language has source, because whether a shared type clashes should not
/// depend on which shell you build today.
fn validate_shell_handlers(
    registry: &Registry,
    handlers: &[&'static ShellHandler],
) -> Result<(), TypeGenError> {
    use facet_generate::reflection::format::Namespace;

    let mut seen: Vec<&str> = Vec::with_capacity(handlers.len());
    for handler in handlers {
        if seen.contains(&handler.name) {
            return Err(TypeGenError::Generation(format!(
                "two shell handlers are called `{}`. Register each capability's handler once.",
                handler.name
            )));
        }
        seen.push(handler.name);

        for name in [
            handler.name.to_string(),
            handler.protocol_name(),
            handler.csharp_interface_name(),
        ] {
            if RESERVED_TYPE_NAMES.contains(&name.as_str()) {
                return Err(TypeGenError::Generation(format!(
                    "shell handler `{}` needs the name `{name}`, which is generated for the shell API. Rename the handler.",
                    handler.name
                )));
            }
        }
    }

    for registered in registry.keys() {
        if registered.namespace != Namespace::Root {
            continue;
        }
        for handler in handlers {
            let claimed = registered.name == handler.name
                || registered.name == handler.protocol_name()
                || registered.name == handler.csharp_interface_name();
            if claimed {
                return Err(TypeGenError::Generation(format!(
                    "`{}` is claimed by the `{}` shell handler, so a shared type cannot be called that. Rename the type with `#[facet(rename = \"...\")]`.",
                    registered.name, handler.name
                )));
            }
        }
    }

    Ok(())
}

/// The base name of the file a package or namespace writes its own module
/// into, for the languages that derive it from the last dotted segment.
///
/// Swift does not: its module is a directory of the package's own name, so it
/// passes the package name through as it is.
fn module_file_stem(package_name: &str) -> String {
    use heck::ToUpperCamelCase as _;

    package_name
        .rsplit('.')
        .next()
        .unwrap_or(package_name)
        .to_upper_camel_case()
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
    ///     &Config::builder("App", output_root.join("swift"))
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn swift(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Swift types");
        // Swift's module is a directory named for the package, holding a file
        // of the same name.
        self.check(Some(&config.package_name))?;
        let path = config.out_dir.join(&config.package_name);

        fs::create_dir_all(&path)?;

        let mut installer = swift::Installer::new(&config.package_name, &path)
            .plugin(BincodePlugin)
            .plugin(OperationKindPlugin::new(&self.effects));
        if self.handlers {
            installer = installer.plugin(EffectHandlerPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin(&config.package_name) {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_handlers) = self.shell_handler_plugin(&config.package_name) {
            installer = installer.plugin(shell_handlers);
        }
        installer
            .external_packages(&config.external_packages)
            .platforms(&config.platforms)
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
        self.check(Some(&module_file_stem(&config.package_name)))?;
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        let mut installer = kotlin::Installer::new(&config.package_name, &config.out_dir)
            .plugin(BincodePlugin)
            .plugin(OperationKindPlugin::new(&self.effects));
        if self.handlers {
            installer = installer.plugin(EffectHandlerPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin(&config.package_name) {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_handlers) = self.shell_handler_plugin(&config.package_name) {
            installer = installer.plugin(shell_handlers);
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
        self.check(Some(&module_file_stem(&config.package_name)))?;
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        let mut installer = csharp::Installer::new(&config.package_name, &config.out_dir)
            .plugin(BincodePlugin)
            .plugin(OperationKindPlugin::new(&self.effects));
        if self.handlers {
            installer = installer.plugin(EffectHandlerPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin(&config.package_name) {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_handlers) = self.shell_handler_plugin(&config.package_name) {
            installer = installer.plugin(shell_handlers);
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
    ///     &Config::builder("app", output_root.join("typescript"))
    ///     .build()
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    /// # Errors
    /// Errors that can occur during type generation.
    pub fn typescript(&self, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating TypeScript types");
        self.check(None)?;
        fs::create_dir_all(&config.out_dir)?;
        let output_dir = &config.out_dir;

        let mut installer = typescript::Installer::new(&config.package_name, output_dir)
            .plugin(BincodePlugin)
            .plugin(OperationKindPlugin::new(&self.effects));
        if self.handlers {
            installer = installer.plugin(EffectHandlerPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin(&config.package_name) {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_handlers) = self.shell_handler_plugin(&config.package_name) {
            installer = installer.plugin(shell_handlers);
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

    /// The `Event` and `ViewModel` of the first registered app, or `None` if no
    /// app was registered.
    #[must_use]
    pub const fn app(&self) -> Option<&AppMeta> {
        self.app.as_ref()
    }

    /// Turns off emission of the effect handler API — `EffectSink`,
    /// `EffectHandler` and `EffectDispatcher` — and, because they are built
    /// on the dispatcher, the generated `CoreBridge` and `Core`.
    ///
    /// Use this if your shell dispatches effects by hand and the handler
    /// declarations are in the way. The `OperationKind` type and the
    /// per-effect accessor stay: a shell that resolves requests itself is
    /// exactly the shell that needs to know whether to resolve each one
    /// never, once, or many times, and the accessor is a plain method on the
    /// effect, as usable from a hand-written `switch` as from the dispatcher.
    /// Shipped shell handlers stay for the same reason.
    #[must_use]
    pub const fn without_effect_handlers(mut self) -> Self {
        self.handlers = false;
        self
    }

    /// Turns off emission of `CoreBridge` and `Core`, keeping the effect
    /// handler API.
    ///
    /// Use this if your shell drives `EffectDispatcher` itself. To turn the
    /// handler API off as well, use
    /// [`without_effect_handlers`](Self::without_effect_handlers), which
    /// implies this.
    #[must_use]
    pub const fn without_core(mut self) -> Self {
        self.core = false;
        self
    }

    /// Names the `BoltFFI` bindings the generated `Core` should bridge to, so
    /// that an `FfiBridge` and a one-argument `Core` constructor are generated
    /// as well.
    ///
    /// Opt in per language — a language [`BoltFfi`] does not name is generated
    /// exactly as it is without this call.
    ///
    /// ```rust
    /// # use crux_core::type_generation::facet::{BoltFfi, TypeRegistry};
    /// let typegen = TypeRegistry::new()
    ///     .build()?
    ///     .boltffi(BoltFfi::new().swift("Shared").kotlin());
    /// # let _ = typegen;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    #[must_use]
    pub fn boltffi(mut self, boltffi: BoltFfi) -> Self {
        self.boltffi = Some(boltffi);
        self
    }

    /// Everything the configuration is checked for before a language is
    /// generated. The builder methods are infallible, so this is where a
    /// configuration that cannot work is reported.
    ///
    /// `module_file` is what the module calls its own source file in this
    /// language, which a companion file beside it must not take. TypeScript's
    /// module is one file with the sources appended to it, so it has no
    /// companion files and passes `None`.
    fn check(&self, module_file: Option<&str>) -> Result<(), TypeGenError> {
        self.check_boltffi()?;
        if let Some(module_file) = module_file {
            self.check_shell_handler_files(module_file)?;
        }
        Ok(())
    }

    /// The bridge is generated as part of `Core`, so asking for one without a
    /// `Core` to put it in is a mistake worth saying out loud — silently
    /// generating nothing would leave the shell with a missing constructor and
    /// no clue why.
    fn check_boltffi(&self) -> Result<(), TypeGenError> {
        if self.boltffi.is_none() {
            return Ok(());
        }

        let reason = if !self.handlers {
            Some("`without_effect_handlers()` turns the generated `Core` off")
        } else if !self.core {
            Some("`without_core()` turns the generated `Core` off")
        } else if self.app.is_none() {
            Some("no app was registered, so there is no `Core` to construct")
        } else if !self.emits_core() {
            Some("the first registered effect has no render variant, so no `Core` is generated")
        } else {
            None
        };

        reason.map_or(Ok(()), |reason| {
            Err(TypeGenError::Generation(format!(
                "`boltffi` generates the bridge the generated `Core` is built with, but {reason}"
            )))
        })
    }

    /// Whether the first registered effect has a render variant, which is what
    /// the `Core` plugin needs to emit anything.
    fn emits_core(&self) -> bool {
        self.effects
            .first()
            .is_some_and(|effect| effect.variants.iter().any(|variant| variant.render))
    }

    /// The plugin that emits `CoreBridge` and `Core`, if it should be emitted
    /// at all.
    ///
    /// `Core` is built on the dispatcher and names the app's `Event` and
    /// `ViewModel`, so it needs the handler API and a registered app.
    fn core_plugin(&self, package_name: &str) -> Option<CorePlugin> {
        if !(self.handlers && self.core) {
            return None;
        }
        self.app
            .clone()
            .map(|app| CorePlugin::new(&self.effects, app, self.boltffi.clone(), package_name))
    }

    /// A handler's source is written beside the module as `<Name>.swift`,
    /// `<Name>.kt` or `<Name>.cs`, and the module names its own file after the
    /// last segment of the package or namespace. A package called `Crux.Http`
    /// would therefore have the `Http` handler overwrite `Crux/Http/Http.cs` —
    /// the types themselves — so it is rejected while both files still exist.
    ///
    /// This is the one check that needs the [`Config`], so it is made here and
    /// not in [`validate_shell_handlers`].
    fn check_shell_handler_files(&self, module_file: &str) -> Result<(), TypeGenError> {
        for handler in &self.shell_handlers {
            if handler.name == module_file {
                return Err(TypeGenError::Generation(format!(
                    "the `{name}` shell handler is written beside the generated module, which names its own file `{name}` too — the package or namespace ends in `{name}`. Generate into a package of another name, or use a capability whose handler is named differently.",
                    name = handler.name
                )));
            }
        }

        Ok(())
    }

    /// The plugin that emits the registered shipped handlers, if any were
    /// registered.
    ///
    /// It is installed outside the `handlers` flag: a shipped handler is a
    /// plain type, as usable from a hand-written `switch` on `Effect` as from
    /// the generated dispatcher, so
    /// [`without_effect_handlers`](Self::without_effect_handlers) does not
    /// take it away.
    fn shell_handler_plugin(&self, package_name: &str) -> Option<ShellHandlerPlugin> {
        use facet_generate::reflection::format::Namespace;

        if self.shell_handlers.is_empty() {
            return None;
        }
        let last_type = self
            .registry
            .keys()
            .rfind(|name| name.namespace == Namespace::Root)
            .cloned();
        Some(ShellHandlerPlugin::new(
            &self.shell_handlers,
            package_name,
            last_type,
        ))
    }
}
