//! Shell protocols: the shell side of a capability's contract, which crux
//! emits into the app's generated module when the app uses the capability.
//!
//! A protocol is the handler interface that a shell implements for a
//! capability (`KeyValueHandler`, `HttpHandler`), with the rules that every
//! implementation must follow. The capability crate writes it once in each
//! language, and the protocol uses only the standard library of that language.
//! A protocol method for a stream can also name the generic `EffectSink`
//! (`IEffectSink` in C#). Crux declares the sink in the root module of every
//! package that has a protocol.
//!
//! The app does not register a protocol. Each operation of the capability
//! names the protocol with `#[operation(.., protocol = path)]`, and the
//! generated `register_types_facet` of the operation calls
//! [`TypeRegistry::shell_protocol`]. The `#[effect]` macro calls
//! `register_types_facet` for every variant, so an app that uses one operation
//! of a capability gets the protocol of that capability.
//!
//! The protocol follows the `facet_typegen` feature of `crux_core`. The
//! generated `register_types_facet` is inside
//! `crux_core::__crux_core_facet_typegen_items!`, and the capability crate puts
//! its protocol items inside the same macro. Thus an app that turns on
//! `crux_core/facet_typegen` gets the protocol, and the capability crate needs
//! no feature for it.
//!
//! ```rust,ignore
//! // In the capability crate, in `src/shell.rs`
//! crux_core::__crux_core_facet_typegen_items! {
//!     pub static PROTOCOL: ShellProtocol = ShellProtocol::new("KeyValue")
//!         .rust_types(register_types)
//!         .swift(include_str!("../shell/swift/KeyValue.swift"), &["KeyValueHandler"])
//!         .kotlin(include_str!("../shell/kotlin/KeyValue.kt"), &["KeyValueHandler"])
//!         .typescript(include_str!("../shell/typescript/keyValue.ts"), &["KeyValueHandler"])
//!         .csharp(include_str!("../shell/csharp/KeyValue.cs"), &["IKeyValueHandler"]);
//! }
//!
//! // In `src/operation.rs`
//! #[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
//! #[operation(request, output = ValueResult, protocol = crate::shell::PROTOCOL)]
//! pub struct GetValue {
//!     pub key: String,
//! }
//! ```

use std::result::Result;

use crate::type_generation::facet::{Language, TypeGenError, TypeRegistry};

/// How a protocol registers the Rust types that its sources name.
///
/// This is the same shape as
/// [`Export::register_types`](crate::type_generation::facet::Export::register_types)
/// and as the `register_types_facet` function of an operation. Thus a
/// capability with one operation can name that function directly.
pub type RegisterTypes = fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>;

/// The shell side of a capability's contract, in the languages that the
/// capability writes it for.
///
/// A capability has one protocol, not one for each operation. The operations
/// of a capability share state (an HTTP client, a key-value store, a timer
/// table), so a shell implements them together.
///
/// Make a protocol with [`new`](Self::new) and the `const` builders, so that a
/// crate can declare it in a `static`. A language for which the protocol has no
/// source gets nothing. The shell then writes the interface itself.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShellProtocol {
    /// The name of the capability, in `UpperCamelCase`. It names the companion
    /// file that holds the source (`Http.swift`, `Http.kt`, `Http.cs`), and
    /// crux uses it to find a protocol that is already registered.
    ///
    /// [`TypeRegistry::build`] checks the name. It must start with an ASCII
    /// letter and contain only ASCII letters, digits and `_`. It must not be a
    /// name that Windows reserves for a device, or the name of a file that the
    /// generated shell API writes (`FfiBridge`). Two protocols must not have
    /// the same name, even in different case. A
    /// [`ShellExtension`](crate::type_generation::facet::ShellExtension) that
    /// implements the protocol has the same name, because both have the name
    /// of the capability.
    pub name: &'static str,
    /// Registers the Rust types that the sources name. [`TypeRegistry::shell_protocol`]
    /// runs it.
    ///
    /// A shell implements the whole protocol, so crux must generate every
    /// operation and output of the capability, including the ones that this app
    /// never sends. The capability registers them through this function, so the
    /// app does not need to know which types they are.
    pub rust_types: Option<RegisterTypes>,
    pub swift: Option<ProtocolSource>,
    pub kotlin: Option<ProtocolSource>,
    pub typescript: Option<ProtocolSource>,
    pub csharp: Option<ProtocolSource>,
}

/// The source of a protocol in one language, and the top-level names that the
/// source declares.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ProtocolSource {
    /// The source text. Crux writes it unchanged into the generated module,
    /// after the module header that the language needs. The source must put
    /// its own imports (`Foundation`, `java.net.*`, `System.*`) at the top.
    pub source: &'static str,
    /// The top-level names that the source declares in the generated module,
    /// for example `HttpHandler` (`IHttpHandler` in C#).
    ///
    /// The source shares a module with the app's types and the generated
    /// shell API. Thus [`TypeRegistry::build`] checks each name against the
    /// names that the shell API reserves, the names that the module imports
    /// from the serialization runtime, the namespaces of the app's types, the
    /// names that other protocols and shell extensions declare, and the types
    /// in the root namespace. A clash is then a generation error, not a module
    /// that does not compile.
    pub foreign_types: &'static [&'static str],
}

impl ProtocolSource {
    const fn new(source: &'static str, foreign_types: &'static [&'static str]) -> Self {
        Self {
            source,
            foreign_types,
        }
    }
}

impl ShellProtocol {
    /// Makes a protocol that has no source yet. A capability gives it its own
    /// name: `Http`, `Time`, `KeyValue`. Add a source for each language with
    /// the builders.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            rust_types: None,
            swift: None,
            kotlin: None,
            typescript: None,
            csharp: None,
        }
    }

    /// Sets the function that registers the types that the sources name. See
    /// [`rust_types`](Self::rust_types).
    ///
    /// Use a plain `fn` item, or a closure that captures nothing, so that the
    /// declaration is still a `static`.
    #[must_use]
    pub const fn rust_types(mut self, rust_types: RegisterTypes) -> Self {
        self.rust_types = Some(rust_types);
        self
    }

    /// Sets the Swift source and the top-level names that it declares.
    #[must_use]
    pub const fn swift(
        mut self,
        source: &'static str,
        foreign_types: &'static [&'static str],
    ) -> Self {
        self.swift = Some(ProtocolSource::new(source, foreign_types));
        self
    }

    /// Sets the Kotlin source and the top-level names that it declares.
    #[must_use]
    pub const fn kotlin(
        mut self,
        source: &'static str,
        foreign_types: &'static [&'static str],
    ) -> Self {
        self.kotlin = Some(ProtocolSource::new(source, foreign_types));
        self
    }

    /// Sets the TypeScript source and the top-level names that it declares.
    #[must_use]
    pub const fn typescript(
        mut self,
        source: &'static str,
        foreign_types: &'static [&'static str],
    ) -> Self {
        self.typescript = Some(ProtocolSource::new(source, foreign_types));
        self
    }

    /// Sets the C# source and the top-level names that it declares.
    #[must_use]
    pub const fn csharp(
        mut self,
        source: &'static str,
        foreign_types: &'static [&'static str],
    ) -> Self {
        self.csharp = Some(ProtocolSource::new(source, foreign_types));
        self
    }

    /// The source in `language`, if the protocol has one.
    pub(crate) const fn source(&self, language: Language) -> Option<&ProtocolSource> {
        match language {
            Language::Swift => self.swift.as_ref(),
            Language::Kotlin => self.kotlin.as_ref(),
            Language::TypeScript => self.typescript.as_ref(),
            Language::CSharp => self.csharp.as_ref(),
        }
    }

    /// The top-level names that the sources declare, in all languages,
    /// without repeats.
    pub(crate) fn foreign_types(&self) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        for source in [&self.swift, &self.kotlin, &self.typescript, &self.csharp]
            .into_iter()
            .flatten()
        {
            for name in source.foreign_types {
                if !names.contains(name) {
                    names.push(name);
                }
            }
        }
        names
    }
}
