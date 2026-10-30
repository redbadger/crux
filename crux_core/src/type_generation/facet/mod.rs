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
//! ## Shell extensions
//!
//! A crate can ship source for the shell as a [`ShellExtension`]. An extension
//! is one language, and its source is a directory with a `common/` set of
//! files and a set for each platform that needs files of its own. Name the
//! extensions you want with [`TypeRegistry::shell_extension`], and crux emits
//! them into the generated module. The main example is a capability's shell
//! handler, which is the shell side of its protocol. It has the rules that
//! turn a platform response into its operation's output.
//!
//! ```rust
//! use crux_core::type_generation::facet::{
//!     Platform, ShellExtension, TypeGenError, TypeRegistry,
//! };
//! # use include_dir::{Dir, DirEntry, File};
//! # const URL_SESSION: Dir<'static> = Dir::new("", &[DirEntry::Dir(Dir::new(
//! #     "common",
//! #     &[DirEntry::File(File::new(
//! #         "common/Http.swift",
//! #         b"public protocol HttpHandler: Sendable {}\n",
//! #     ))],
//! # ))]);
//!
//! // Normally `crux_http::shell::swift::URL_SESSION`, declared by the
//! // capability crate, with
//! // `include_dir!("$CARGO_MANIFEST_DIR/shell/swift/url_session")`.
//! static EXTENSION: ShellExtension = ShellExtension::swift("Http", URL_SESSION)
//!     .rust_types(register_types)
//!     .platforms(&[Platform::Apple, Platform::Linux])
//!     .foreign_types(&["HttpHandler"]);
//!
//! // A real capability registers each of its operations here, so that the
//! // types its sources name exist even when the app never sends them:
//! // `HttpRequest::register_types_facet(registry)`.
//! fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
//!     Ok(registry)
//! }
//!
//! let typegen = TypeRegistry::new().shell_extension(&EXTENSION)?.build()?;
//! # let _ = typegen;
//! # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
//! ```
//!
//! Registering the extension also registers the types its sources name, so
//! crux generates the whole capability even when the app's `Effect` uses only
//! part of it. No generated code calls the shipped implementation. Your
//! handler holds an instance and delegates to it, one line per operation.
//!
//! [`CodeGenerator::generate`] generates the language of a [`Platform`], with
//! the shell extensions for that platform. For example, an Android shell uses
//! `typegen.generate(Platform::Android, &config)`. The `swift`, `kotlin`,
//! `typescript` and `csharp` methods generate for the language's default
//! platform: Apple for Swift, the JVM for Kotlin, the web for TypeScript and
//! .NET for C#.
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
use self::plugins::{
    COMPANION_FILE_STEMS, CorePlugin, EffectHandlerPlugin, FFI_BRIDGE_FILE, OperationKindPlugin,
    RequestIdPlugin, ShellExtensionPlugin,
};
use self::shell_handler::SourceFile;
pub use self::shell_handler::{Dir, Language, Platform, RegisterTypes, ShellExtension};
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
    "EffectKind",
    "RequestId",
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
    // wired up. The type is named like the file it is written into.
    FFI_BRIDGE_FILE,
];

/// Names the generated root module imports from the serialization runtime, in
/// at least one language: the Swift `Serde` module and what it exports, the
/// Kotlin imports from `com.novi.serde` and `com.novi.bincode`, the
/// TypeScript `Serializer` and `Deserializer`, and the C# runtime interfaces.
/// A shell extension declaring one of these would shadow the import or clash
/// with it.
const RUNTIME_IMPORT_NAMES: &[&str] = &[
    "Serde",
    "Serializer",
    "Deserializer",
    "ISerializer",
    "IDeserializer",
    "BincodeSerializer",
    "BincodeDeserializer",
    "SerializationError",
    "DeserializationError",
    "Bytes",
    "Int128",
    "UInt128",
];

/// File names Windows reserves for devices, whatever the extension, so a
/// companion file cannot be called one of them.
const WINDOWS_DEVICE_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

pub struct TypeRegistry {
    builder: RegistryBuilder,
    effects: Vec<EffectMeta>,
    app: Option<AppMeta>,
    shell_extensions: Vec<&'static ShellExtension>,
}

pub struct CodeGenerator {
    registry: Registry,
    effects: Arc<[EffectMeta]>,
    app: Option<AppMeta>,
    handlers: bool,
    core: bool,
    boltffi: Option<BoltFfi>,
    shell_extensions: Vec<&'static ShellExtension>,
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
            shell_extensions: Vec::new(),
        }
    }

    /// Registers a shell extension, and the types its sources name.
    ///
    /// An extension is one language. Register one for each language you
    /// generate. When crux generates a language, it emits each registered
    /// extension of that language that lists the platform it generates for
    /// (see [`CodeGenerator::generate`]). It emits the files in the extension's
    /// `common/` directory and in the platform's directory, verbatim. Swift,
    /// Kotlin and C# get one file beside the module for each source file, with
    /// the same name. TypeScript gets the sources after the types, in the
    /// order of their paths.
    ///
    /// ```rust,ignore
    /// let typegen = TypeRegistry::new()
    ///     .register_app::<Weather>()?
    ///     .shell_extension(&crux_http::shell::swift::URL_SESSION)?
    ///     .shell_extension(&crux_http::shell::kotlin::URL_CONNECTION)?
    ///     .build()?;
    /// typegen.generate(Platform::Android, &config)?;
    /// ```
    ///
    /// A capability's shell handler is the main example: a `HttpHandler`
    /// protocol and an implementation of it. No generated code calls it. The
    /// shell's handler holds an instance and delegates to it, one line per
    /// operation.
    ///
    /// A capability's handler implements the whole capability, so registering
    /// it registers every type it names, including the operations this app
    /// never sends. The extension says which those are, with
    /// [`ShellExtension::rust_types`]. Crux registers them once for each
    /// capability name, however many languages you register.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if one of the extension's own types cannot
    /// be registered. [`build`](Self::build) reports these problems:
    ///
    /// * a source directory that is not laid out correctly
    /// * a platform list that is not correct
    /// * a declared name that clashes with the generated shell API, another
    ///   capability's extension or a type of yours.
    ///
    /// When the generator generates a language, it reports the problems that
    /// depend on the language and platform.
    pub fn shell_extension(
        &mut self,
        extension: &'static ShellExtension,
    ) -> Result<&mut Self, TypeGenError> {
        let registered = self
            .shell_extensions
            .iter()
            .any(|earlier| earlier.name == extension.name);
        self.shell_extensions.push(extension);

        if !registered && let Some(register) = extension.rust_types {
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
    /// registered type, effect variant or shell extension claims a name that
    /// is already taken.
    pub fn build(&mut self) -> Result<CodeGenerator, TypeGenError> {
        let builder = std::mem::take(&mut self.builder);
        let effects: Arc<[EffectMeta]> = std::mem::take(&mut self.effects).into();
        let shell_extensions = std::mem::take(&mut self.shell_extensions);
        let registry = builder
            .build()
            .map_err(|e| TypeGenError::Generation(e.to_string()))?;

        if !effects.is_empty() {
            validate_names(&registry, &effects)?;
        }
        validate_shell_extensions(&registry, &shell_extensions)?;

        Ok(CodeGenerator {
            registry,
            effects,
            app: self.app.clone(),
            handlers: true,
            core: true,
            boltffi: None,
            shell_extensions,
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

/// Rejects registered shell extensions that cannot be used.
///
/// Crux checks the platform list of each extension and the layout of its
/// source directory. It also checks the name of each source file, because
/// crux writes each file beside the module under its own name. Crux checks
/// the names in `foreign_types` against the generated shell API, the
/// serialization runtime's imports, the app's namespaces, the names that
/// extensions of other capabilities declare, and the types registered in the
/// root namespace.
///
/// Crux makes these checks for every registered extension, whatever language
/// and platform it generates. Whether a shared type clashes does not depend on
/// which shell you build today. Crux makes the checks that depend on the
/// platform (which files it emits together) when it generates a language.
fn validate_shell_extensions(
    registry: &Registry,
    extensions: &[&'static ShellExtension],
) -> Result<(), TypeGenError> {
    use facet_generate::reflection::format::Namespace;
    use heck::ToUpperCamelCase as _;

    let namespaces: Vec<&str> = registry
        .keys()
        .filter_map(|registered| match &registered.namespace {
            Namespace::Named(namespace) => Some(namespace.as_str()),
            Namespace::Root => None,
        })
        .collect();

    for (index, extension) in extensions.iter().enumerate() {
        let name = extension.name;

        validate_extension_name(name)?;
        validate_extension_platforms(extension)?;
        validate_extension_sources(extension)?;

        for declared in extension.foreign_types {
            if RESERVED_TYPE_NAMES.contains(declared) {
                return Err(TypeGenError::Generation(format!(
                    "shell extension `{name}` needs the name `{declared}`, which is reserved for other use in the shell API. Please choose a different name for the extension's type."
                )));
            }

            if RUNTIME_IMPORT_NAMES.contains(declared) {
                return Err(TypeGenError::Generation(format!(
                    "shell extension `{name}` needs the name `{declared}`, which the generated module imports from the serialization runtime. Please choose a different name for the extension's type."
                )));
            }

            if namespaces.iter().any(|namespace| {
                namespace == declared || namespace.to_upper_camel_case() == *declared
            }) {
                return Err(TypeGenError::Generation(format!(
                    "shell extension `{name}` needs the name `{declared}`, which is the name of a namespace the app's types are generated into. Please choose a different name for the extension's type, or rename the namespace."
                )));
            }

            // The extensions of one capability, one for each language or
            // platform, declare the same names. Only another capability's
            // declaration clashes.
            if let Some(earlier) = extensions[..index]
                .iter()
                .find(|earlier| earlier.name != name && earlier.foreign_types.contains(declared))
            {
                return Err(TypeGenError::Generation(format!(
                    "shell extensions `{}` and `{name}` both declare `{declared}`, so the generated module would declare it twice. Only one of them can be registered.",
                    earlier.name
                )));
            }
        }
    }

    for registered in registry.keys() {
        if registered.namespace != Namespace::Root {
            continue;
        }
        for extension in extensions {
            if extension.foreign_types.contains(&registered.name.as_str()) {
                return Err(TypeGenError::Generation(format!(
                    "`{}` is declared by the `{}` shell extension, so a shared type cannot be called that. Rename the type with `#[facet(rename = \"...\")]`.",
                    registered.name, extension.name
                )));
            }
        }
    }

    Ok(())
}

/// Whether `name` starts with an ASCII letter and contains only ASCII letters,
/// digits and `_`.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Rejects an extension name that is not an identifier. The name is the
/// capability's, and crux uses it in error messages and to group the
/// extensions of one capability.
fn validate_extension_name(name: &str) -> Result<(), TypeGenError> {
    if !is_identifier(name) {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has a name that is not an identifier. The name of a capability has to start with an ASCII letter and contain only ASCII letters, digits and `_`."
        )));
    }

    Ok(())
}

/// Rejects an extension that lists no platforms, or lists a platform of
/// another language.
fn validate_extension_platforms(extension: &ShellExtension) -> Result<(), TypeGenError> {
    let name = extension.name;
    let language = extension.language.display_name();

    if extension.platforms.is_empty() {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` ({language}) lists no platforms, so crux never emits it. Name the platforms its source runs on with `.platforms(..)`."
        )));
    }

    for platform in extension.platforms {
        if platform.language() != extension.language {
            return Err(TypeGenError::Generation(format!(
                "shell extension `{name}` is {language} source, but it lists the {platform:?} platform, which is a {} platform.",
                platform.language().display_name()
            )));
        }
    }

    Ok(())
}

/// Rejects a source directory that is not laid out as source sets, and a
/// source file that cannot be written beside the module.
///
/// The root can contain only `common/` and the directories of the language's
/// platforms. Otherwise crux would skip a misspelled directory, such as
/// `andriod/`, and report nothing. Each of those directories can contain only
/// files.
fn validate_extension_sources(extension: &'static ShellExtension) -> Result<(), TypeGenError> {
    use include_dir::DirEntry;

    let name = extension.name;
    let language = extension.language;
    let platforms = language.platforms();

    for entry in extension.sources.entries() {
        let path = entry.path().to_string_lossy();
        if path.starts_with('.') {
            continue;
        }
        let DirEntry::Dir(dir) = entry else {
            return Err(TypeGenError::Generation(format!(
                "shell extension `{name}` has the file `{path}` at the root of its source directory. Put each file in `common/` or in the directory of a platform."
            )));
        };
        let known = path == shell_handler::COMMON
            || platforms
                .iter()
                .any(|platform| platform.directory() == path);
        if !known {
            let expected: Vec<String> = std::iter::once(shell_handler::COMMON)
                .chain(platforms.iter().map(|platform| platform.directory()))
                .map(|directory| format!("`{directory}/`"))
                .collect();
            return Err(TypeGenError::Generation(format!(
                "shell extension `{name}` has the directory `{path}/`, which crux does not know. The source directory of a {} extension can contain only {}.",
                language.display_name(),
                expected.join(", ")
            )));
        }

        for entry in dir.entries() {
            let DirEntry::File(file) = entry else {
                return Err(TypeGenError::Generation(format!(
                    "shell extension `{name}` has the directory `{}/`. The directory `{path}/` can contain only files.",
                    entry.path().to_string_lossy()
                )));
            };
            validate_source_file(extension, file)?;
        }
    }

    Ok(())
}

/// Rejects a source file that is not UTF-8 text in the extension's language,
/// or whose name cannot be written beside the module in every language and
/// on every file system.
fn validate_source_file(
    extension: &'static ShellExtension,
    file: &'static include_dir::File<'static>,
) -> Result<(), TypeGenError> {
    let name = extension.name;
    let Some(source) = SourceFile::new(file) else {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{}`, whose path is not UTF-8.",
            file.path().to_string_lossy()
        )));
    };
    if source.is_hidden() {
        return Ok(());
    }
    let path = source.path;
    let file_extension = extension.language.file_extension();

    if source.extension != file_extension {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{path}`, which is not a {} source file. Each file has to have the extension `.{file_extension}`.",
            extension.language.display_name()
        )));
    }

    if !is_identifier(source.stem) {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{path}`, whose name cannot be used as a file name in every language. The name before `.{file_extension}` has to start with an ASCII letter and contain only ASCII letters, digits and `_`."
        )));
    }

    if WINDOWS_DEVICE_NAMES
        .iter()
        .any(|device| device.eq_ignore_ascii_case(source.stem))
    {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{path}`, whose name Windows reserves for a device, so the file could not be written there. Please rename the file."
        )));
    }

    if let Some(reserved) = COMPANION_FILE_STEMS
        .iter()
        .find(|reserved| reserved.eq_ignore_ascii_case(source.stem))
    {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{path}`, which would be written over the generated `{reserved}` file. That file is reserved for other use in the shell API. Please rename the file."
        )));
    }

    if source.contents.is_none() {
        return Err(TypeGenError::Generation(format!(
            "shell extension `{name}` has the file `{path}`, which is not UTF-8 text."
        )));
    }

    Ok(())
}

/// The base name of the file a package or namespace writes its own module
/// into, for the languages that derive it from the last dotted segment
/// (Kotlin and C#).
///
/// Swift does not: see [`swift_module_file_stem`].
fn module_file_stem(package_name: &str) -> String {
    use heck::ToUpperCamelCase as _;

    package_name
        .rsplit('.')
        .next()
        .unwrap_or(package_name)
        .to_upper_camel_case()
}

/// The base name of the file the Swift package writes its own module into.
///
/// facet-generate's Swift installer names the module's directory and its
/// file after the whole package name in `UpperCamelCase`, so a package called
/// `shared_types` writes `Sources/SharedTypes/SharedTypes.swift`.
fn swift_module_file_stem(package_name: &str) -> String {
    use heck::ToUpperCamelCase as _;

    package_name.to_upper_camel_case()
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeGenerator {
    /// Generates types for the language of `platform`, with the shell
    /// extensions for that platform.
    ///
    /// For each capability with registered extensions of the language, crux
    /// emits the extension that lists `platform`. From that extension, it
    /// emits the files in `common/` and in the platform's directory. One
    /// generator can generate for several platforms, so an app can generate
    /// Kotlin for the JVM and for Android into two output directories.
    ///
    /// ```rust
    /// # use crux_core::type_generation::facet::{Config, Platform, TypeRegistry};
    /// # use std::env::temp_dir;
    /// # let typegen = TypeRegistry::new().build()?;
    /// # let output_root = temp_dir().join("crux_core_typegen_doctest");
    /// typegen.generate(
    ///     Platform::Android,
    ///     &Config::builder("com.crux.example", output_root.join("android")).build(),
    /// )?;
    /// # Ok::<(), crux_core::type_generation::facet::TypeGenError>(())
    /// ```
    ///
    /// # Errors
    /// Errors that can occur during type generation. If a capability has
    /// registered extensions of the language but none of them lists
    /// `platform`, or more than one lists it, generation fails.
    pub fn generate(&self, platform: Platform, config: &Config) -> Result<(), TypeGenError> {
        match platform.language() {
            Language::Swift => self.generate_swift(platform, config),
            Language::Kotlin => self.generate_kotlin(platform, config),
            Language::TypeScript => self.generate_typescript(platform, config),
            Language::CSharp => self.generate_csharp(platform, config),
        }
    }

    /// Generates types for Swift on Apple, the default platform of Swift. It
    /// is the same as `generate(Platform::Apple, config)`. To
    /// generate for another platform, use [`generate`](Self::generate).
    ///
    /// For example:
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
        self.generate(Platform::Apple, config)
    }

    fn generate_swift(&self, platform: Platform, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Swift types");
        // Swift's module is a directory named for the package, holding a file
        // of the same name, both in `UpperCamelCase`.
        self.check_boltffi()?;
        let shell_extensions = self.shell_extension_plugin(
            platform,
            &config.package_name,
            Some(&swift_module_file_stem(&config.package_name)),
        )?;
        let path = config.out_dir.join(&config.package_name);

        fs::create_dir_all(&path)?;

        let mut installer =
            swift::Installer::new(&config.package_name, &path).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(OperationKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects))
                .plugin(RequestIdPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin() {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_extensions) = shell_extensions {
            installer = installer.plugin(shell_extensions);
        }
        installer
            .external_packages(&config.external_packages)
            .platforms(&config.platforms)
            .generate(&self.registry)?;

        Ok(())
    }

    /// Generates types for Kotlin on the JVM, the default platform of Kotlin.
    /// It is the same as `generate(Platform::Jvm, config)`. To
    /// generate for another platform, use [`generate`](Self::generate).
    ///
    /// For example:
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
        self.generate(Platform::Jvm, config)
    }

    fn generate_kotlin(&self, platform: Platform, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating Kotlin types");
        self.check_boltffi()?;
        let shell_extensions = self.shell_extension_plugin(
            platform,
            &config.package_name,
            Some(&module_file_stem(&config.package_name)),
        )?;
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        let mut installer =
            kotlin::Installer::new(&config.package_name, &config.out_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(OperationKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects))
                .plugin(RequestIdPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin() {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_extensions) = shell_extensions {
            installer = installer.plugin(shell_extensions);
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

        Ok(())
    }

    /// Generates types for C# on .NET, the default platform of C#. It is the
    /// same as `generate(Platform::DotNet, config)`. To
    /// generate for another platform, use [`generate`](Self::generate).
    ///
    /// For example:
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
        self.generate(Platform::DotNet, config)
    }

    fn generate_csharp(&self, platform: Platform, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating C# types");
        self.check_boltffi()?;
        let shell_extensions = self.shell_extension_plugin(
            platform,
            &config.package_name,
            Some(&module_file_stem(&config.package_name)),
        )?;
        fs::create_dir_all(&config.out_dir)?;

        let package_path = config.package_name.replace('.', "/");

        // remove any existing generated shared types, this ensures that we remove no longer used types
        fs::remove_dir_all(config.out_dir.join(&package_path)).unwrap_or(());

        let mut installer =
            csharp::Installer::new(&config.package_name, &config.out_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(OperationKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects))
                .plugin(RequestIdPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin() {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_extensions) = shell_extensions {
            installer = installer.plugin(shell_extensions);
        }
        installer
            .external_packages(&config.external_packages)
            .generate(&self.registry)?;

        Ok(())
    }

    /// Generates types for TypeScript on the web, the default platform of
    /// TypeScript. It is the same as `generate(Platform::Web, config)`. To
    /// generate for another platform, use [`generate`](Self::generate).
    ///
    /// For example:
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
        self.generate(Platform::Web, config)
    }

    fn generate_typescript(&self, platform: Platform, config: &Config) -> Result<(), TypeGenError> {
        info!("Generating TypeScript types");
        self.check_boltffi()?;
        let shell_extensions = self.shell_extension_plugin(platform, &config.package_name, None)?;
        fs::create_dir_all(&config.out_dir)?;
        let output_dir = &config.out_dir;

        let mut installer =
            typescript::Installer::new(&config.package_name, output_dir).plugin(BincodePlugin);
        if self.handlers {
            installer = installer
                .plugin(OperationKindPlugin::new(&self.effects))
                .plugin(EffectHandlerPlugin::new(&self.effects))
                .plugin(RequestIdPlugin::new(&self.effects));
            if let Some(core) = self.core_plugin() {
                installer = installer.plugin(core);
            }
        }
        if let Some(shell_extensions) = shell_extensions {
            installer = installer.plugin(shell_extensions);
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

    /// Turns off emission of the `OperationKind` and `EffectKind` types, the
    /// `RequestId` decoder, the effect handler API, and — because it is built
    /// on the dispatcher — the generated `Core`.
    ///
    /// Only the types you registered are generated, exactly as before Crux
    /// 0.21. Use this if your shell dispatches effects by hand and the extra
    /// declarations are in the way.
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
    fn core_plugin(&self) -> Option<CorePlugin> {
        if !(self.handlers && self.core) {
            return None;
        }
        self.app
            .clone()
            .map(|app| CorePlugin::new(&self.effects, app, self.boltffi.clone()))
    }

    /// The registered extensions crux emits for `language` on `platform`: for
    /// each capability with an extension of this language, the one extension
    /// that lists the platform. The capabilities are in registration order.
    fn select_shell_extensions(
        &self,
        language: Language,
        platform: Platform,
    ) -> Result<Vec<&'static ShellExtension>, TypeGenError> {
        let of_language: Vec<&'static ShellExtension> = self
            .shell_extensions
            .iter()
            .filter(|extension| extension.language == language)
            .copied()
            .collect();

        let mut capabilities: Vec<&str> = Vec::new();
        for extension in &of_language {
            if !capabilities.contains(&extension.name) {
                capabilities.push(extension.name);
            }
        }

        let mut selected = Vec::new();
        for capability in capabilities {
            let candidates: Vec<&'static ShellExtension> = of_language
                .iter()
                .filter(|extension| extension.name == capability)
                .copied()
                .collect();
            let matching: Vec<&'static ShellExtension> = candidates
                .iter()
                .filter(|extension| extension.platforms.contains(&platform))
                .copied()
                .collect();

            match matching.as_slice() {
                [extension] => selected.push(*extension),
                [] => {
                    let mut listed: Vec<String> = Vec::new();
                    for extension in &candidates {
                        for listed_platform in extension.platforms {
                            let listed_platform = format!("{listed_platform:?}");
                            if !listed.contains(&listed_platform) {
                                listed.push(listed_platform);
                            }
                        }
                    }
                    return Err(TypeGenError::Generation(format!(
                        "the registered {language} extensions of shell extension `{capability}` list the platforms {listed}, but {language} is generated for the {platform:?} platform. Register an extension of `{capability}` that lists {platform:?}, or generate for a platform that the extensions list.",
                        language = language.display_name(),
                        listed = listed.join(", "),
                    )));
                }
                _ => {
                    return Err(TypeGenError::Generation(format!(
                        "{count} registered {language} extensions of shell extension `{capability}` list the {platform:?} platform, so crux cannot choose one of them. Register only one of them.",
                        count = matching.len(),
                        language = language.display_name(),
                    )));
                }
            }
        }

        Ok(selected)
    }

    /// The plugin that emits the shell extensions selected for `platform`, if
    /// any are selected.
    ///
    /// Each source file is written beside the module as a file of the same
    /// name, and the module names its own file after the package or
    /// namespace. `module_file` is that name, so that a source file cannot
    /// overwrite the module's types. For example, a package called `Crux.Http`
    /// writes its types to `Crux/Http/Http.cs`, which a shipped `Http.cs` would
    /// replace. Two source files cannot share a name either. Names are
    /// compared ignoring case, because `Http.swift` overwrites `http.swift` on
    /// APFS and NTFS. TypeScript's module is one file with the sources
    /// appended to it, so it has no files to overwrite and passes `None`.
    ///
    /// The plugin is installed outside the `handlers` flag. An extension's
    /// source is plain code, and a capability's shipped handler is as usable
    /// from a hand-written `switch` on `Effect` as from the generated
    /// dispatcher, so [`without_effect_handlers`](Self::without_effect_handlers)
    /// does not remove it.
    fn shell_extension_plugin(
        &self,
        platform: Platform,
        package_name: &str,
        module_file: Option<&str>,
    ) -> Result<Option<ShellExtensionPlugin>, TypeGenError> {
        use facet_generate::reflection::format::Namespace;

        let language = platform.language();
        let selected = self.select_shell_extensions(language, platform)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let mut files: Vec<(&'static ShellExtension, SourceFile)> = Vec::new();
        for extension in &selected {
            for file in extension.files_for(platform) {
                if let Some(module_file) = module_file
                    && file.stem.eq_ignore_ascii_case(module_file)
                {
                    return Err(TypeGenError::Generation(format!(
                        "the `{name}` shell extension's `{path}` is written beside the generated module, which names its own file `{module_file}`. That is the same name, ignoring case, and one file overwrites the other on a case-insensitive file system. The package or namespace ends in `{module_file}`. Generate into a package of another name, or register an extension without a file of that name.",
                        name = extension.name,
                        path = file.path,
                    )));
                }
                if module_file.is_some()
                    && let Some((earlier, other)) = files
                        .iter()
                        .find(|(_, other)| other.stem.eq_ignore_ascii_case(file.stem))
                {
                    return Err(TypeGenError::Generation(format!(
                        "the `{}` shell extension's `{}` and the `{}` shell extension's `{}` are both written beside the generated module, and their names are the same, ignoring case, so one would overwrite the other. Rename one of the files, or register only one of the extensions.",
                        earlier.name, other.path, extension.name, file.path,
                    )));
                }
                files.push((extension, file));
            }
        }

        let mut dependencies: Vec<String> = Vec::new();
        let mut target_dependencies: Vec<String> = Vec::new();
        for extension in &selected {
            for (entries, union) in [
                (extension.dependencies, &mut dependencies),
                (extension.target_dependencies, &mut target_dependencies),
            ] {
                for entry in entries {
                    let entry = (*entry).to_string();
                    if !union.contains(&entry) {
                        union.push(entry);
                    }
                }
            }
        }

        let last_type = self
            .registry
            .keys()
            .rfind(|name| name.namespace == Namespace::Root)
            .cloned();
        Ok(Some(ShellExtensionPlugin::new(
            files.into_iter().map(|(_, file)| file).collect(),
            dependencies,
            target_dependencies,
            package_name,
            last_type,
        )))
    }
}
