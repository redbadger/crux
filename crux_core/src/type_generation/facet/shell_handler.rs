//! Shell handlers a capability crate ships with its types.
//!
//! A capability knows how its protocol works — which `URLError` is a timeout,
//! what a cleared timer answers with — and today every shell re-derives those
//! rules from the book. A capability can instead ship the shell side of its
//! protocol as source: a Swift, Kotlin, TypeScript and C# implementation,
//! versioned with the crate, emitted into the app's own generated module when
//! the app asks for it.
//!
//! ```rust,ignore
//! // In the capability crate
//! #[cfg(feature = "facet_typegen")]
//! pub static HTTP: ShellHandler = ShellHandler::new("Http")
//!     .types(register_types)
//!     .swift(ShellSource::stdlib(include_str!("../shell/swift/Http.swift")))
//!     .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt")))
//!     .typescript(ShellSource::stdlib(include_str!("../shell/typescript/http.ts")))
//!     .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Http.cs")));
//!
//! fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
//!     HttpRequest::register_types_facet(registry)
//! }
//! ```
//!
//! Nothing is emitted for a capability the app did not name with
//! [`TypeRegistry::shell_handler`](crate::type_generation::facet::TypeRegistry::shell_handler),
//! and nothing generated ever calls the shipped code: the app's handler holds
//! an instance and delegates to it, one line per operation. That keeps both
//! decisions — whether to use the shipped implementation, and which platform
//! client to use it with — in the shell, where they belong.

use std::result::Result;

use crate::type_generation::facet::{TypeGenError, TypeRegistry};

/// How a capability registers the types its shipped sources name.
///
/// The same shape as
/// [`Export::register_types`](crate::type_generation::facet::Export::register_types)
/// and as an operation's own `register_types_facet`, so a capability with one
/// operation can name that directly.
pub type RegisterTypes = fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>;

/// The shell handlers one capability ships, in the languages it ships them
/// for.
///
/// One per capability, not per operation: the operations of a capability share
/// state — an HTTP client, a key-value store, a timer table — and the
/// capability is the unit an app would configure or replace.
///
/// Built with [`new`](Self::new) and one `const` builder per language, so that
/// a capability can declare its handler in a `static` and a fifth language is
/// not a breaking change. A language the capability does not ship simply has
/// nothing to emit on that platform; the app implements those methods itself,
/// as it does today.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShellHandler {
    /// `UpperCamelCase`. Names the companion file (`Http.swift`) and the
    /// protocol the source is expected to declare (`HttpHandler`, and
    /// `IHttpHandler` in C#).
    ///
    /// All three names are checked against the generated shell API and the
    /// app's own types when the handler is registered, so a clash is a
    /// generation error rather than a file that fails to compile.
    pub name: &'static str,
    /// Registers the types the shipped sources name, run by
    /// [`TypeRegistry::shell_handler`].
    ///
    /// A shipped source implements the whole capability, so every one of its
    /// operations and outputs has to be generated — including the ones this
    /// app never sends. The capability knows what those are, so it names the
    /// function rather than leaving the app to call it.
    pub types: Option<RegisterTypes>,
    pub swift: Option<ShellSource>,
    pub kotlin: Option<ShellSource>,
    pub typescript: Option<ShellSource>,
    pub csharp: Option<ShellSource>,
}

/// One language's shipped source, and what it needs beyond the platform's
/// standard library.
///
/// [`stdlib`](Self::stdlib) is the constructor; what the source needs is added
/// with the `const` builders, so that the whole declaration is a `static`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ShellSource {
    /// The source text, emitted verbatim into the generated module — after the
    /// module header the language needs, so it carries its own imports
    /// (`Foundation`, `java.net.*`, `System.*`) at the top of the file.
    pub source: &'static str,
    /// Manifest entries the source needs beyond the platform standard library,
    /// in the form facet-generate's `manifest_dependencies` takes: an SPM
    /// `.package(…)` entry in Swift, a Gradle `implementation(…)` line in
    /// Kotlin, a `"name": "version"` pair in TypeScript.
    ///
    /// They reach the manifest only for an app that registers the handler, but
    /// every one of those apps then pays for the library, so the bar is high —
    /// most handlers are [`stdlib`](Self::stdlib).
    ///
    /// A Swift package also needs the target's edge to its product, which is
    /// [`target_dependencies`](Self::target_dependencies). One caveat remains,
    /// facet-generate's: C# manifest entries are not written to the `.csproj`
    /// at all, because its C# installer does not consult plugins. A handler
    /// needing one has to say so in its documentation until facet-generate
    /// grows the hook.
    pub dependencies: &'static [&'static str],
    /// Edges the generated build target needs, Swift only: each entry is
    /// written verbatim into the SPM target's `dependencies:` array, so it has
    /// to be a `Target.Dependency` expression — normally
    /// `.product(name: "…", package: "…")` naming a product of a package
    /// declared in [`dependencies`](Self::dependencies).
    ///
    /// Kotlin and TypeScript name their libraries in the manifest alone, so
    /// they have nothing to put here.
    pub target_dependencies: &'static [&'static str],
}

impl ShellSource {
    /// Source that needs nothing but the platform's standard library, which is
    /// what a shipped handler should normally be.
    #[must_use]
    pub const fn stdlib(source: &'static str) -> Self {
        Self {
            source,
            dependencies: &[],
            target_dependencies: &[],
        }
    }

    /// What the source needs in the generated package's manifest. See
    /// [`dependencies`](Self::dependencies).
    #[must_use]
    pub const fn dependencies(mut self, dependencies: &'static [&'static str]) -> Self {
        self.dependencies = dependencies;
        self
    }

    /// What the source needs on the generated Swift target. See
    /// [`target_dependencies`](Self::target_dependencies).
    #[must_use]
    pub const fn target_dependencies(
        mut self,
        target_dependencies: &'static [&'static str],
    ) -> Self {
        self.target_dependencies = target_dependencies;
        self
    }
}

impl ShellHandler {
    /// A handler that ships no language yet, named for the capability —
    /// `Http`, `Time`, `KeyValue`. Add the languages it ships with the
    /// builders.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            types: None,
            swift: None,
            kotlin: None,
            typescript: None,
            csharp: None,
        }
    }

    /// The function that registers the types the shipped sources name. See
    /// [`types`](Self::types).
    ///
    /// A plain `fn` item, or a closure that captures nothing, so that the
    /// whole declaration is still a `static`.
    #[must_use]
    pub const fn types(mut self, types: RegisterTypes) -> Self {
        self.types = Some(types);
        self
    }

    /// The Swift source this capability ships.
    #[must_use]
    pub const fn swift(mut self, source: ShellSource) -> Self {
        self.swift = Some(source);
        self
    }

    /// The Kotlin source this capability ships.
    #[must_use]
    pub const fn kotlin(mut self, source: ShellSource) -> Self {
        self.kotlin = Some(source);
        self
    }

    /// The TypeScript source this capability ships.
    #[must_use]
    pub const fn typescript(mut self, source: ShellSource) -> Self {
        self.typescript = Some(source);
        self
    }

    /// The C# source this capability ships.
    #[must_use]
    pub const fn csharp(mut self, source: ShellSource) -> Self {
        self.csharp = Some(source);
        self
    }

    /// The protocol the source is expected to declare, e.g. `HttpHandler`.
    #[must_use]
    pub fn protocol_name(&self) -> String {
        format!("{}Handler", self.name)
    }

    /// The same, in C#, where an interface is conventionally `I`-prefixed.
    #[must_use]
    pub fn csharp_interface_name(&self) -> String {
        format!("I{}Handler", self.name)
    }
}
