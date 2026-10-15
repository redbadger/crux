//! Shell extensions: foreign source a crate ships, emitted into the app's
//! generated module when the app registers it.
//!
//! The main use is a capability's shell handler. A capability knows how its
//! protocol works (which `URLError` is a timeout, what a cleared timer answers
//! with), and without a shipped handler every shell re-derives those rules
//! from the book. The capability can instead ship the shell side of its
//! protocol as source: a Swift, Kotlin, TypeScript and C# implementation,
//! versioned with the crate.
//!
//! ```rust,ignore
//! // In the capability crate, in `src/shell.rs`
//! #[cfg(feature = "facet_typegen")]
//! pub static EXTENSION: ShellExtension = ShellExtension::new("Http")
//!     .types(register_types)
//!     .declares(&["HttpHandler", "IHttpHandler"])
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
//! Nothing is emitted for an extension the app did not name with
//! [`TypeRegistry::shell_extension`](crate::type_generation::facet::TypeRegistry::shell_extension),
//! and nothing generated ever calls the shipped code. For a capability's
//! handler, the app's own handler holds an instance and delegates to it, one
//! line per operation. That keeps both decisions (whether to use the shipped
//! implementation, and which platform client to use it with) in the shell,
//! where they belong.

use std::result::Result;

use crate::type_generation::facet::{TypeGenError, TypeRegistry};

/// How an extension registers the types its shipped sources name.
///
/// The same shape as
/// [`Export::register_types`](crate::type_generation::facet::Export::register_types)
/// and as an operation's own `register_types_facet`, so a capability with one
/// operation can name that directly.
pub type RegisterTypes = fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>;

/// Source a crate ships for the shell, in the languages it ships it for.
///
/// A capability ships one extension, holding its shell handler: one per
/// capability, not per operation, because the operations of a capability share
/// state (an HTTP client, a key-value store, a timer table) and the capability
/// is the unit an app would configure or replace.
///
/// Built with [`new`](Self::new) and `const` builders, so that a crate can
/// declare its extension in a `static` and a fifth language is not a breaking
/// change. A language the extension does not ship has nothing to emit on that
/// platform. For a capability's handler, the app implements those methods
/// itself.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShellExtension {
    /// `UpperCamelCase`. Names the companion file the source is written into
    /// (`Http.swift`, `Http.kt`, `Http.cs`), and nothing else.
    ///
    /// Checked when the registry is built: it has to start with an ASCII
    /// letter and contain only ASCII letters, digits and `_`, it cannot be a
    /// name Windows reserves for a device, two extensions cannot share a name,
    /// even in different case, and an extension cannot take the name of a
    /// file the generated shell API writes (`FfiBridge`). A name that matches
    /// the module's own file is rejected when that language is generated.
    pub name: &'static str,
    /// Registers the types the shipped sources name, run by
    /// [`TypeRegistry::shell_extension`].
    ///
    /// A capability's handler implements the whole capability, so every one
    /// of its operations and outputs has to be generated, including the ones
    /// this app never sends. The capability knows what those are, so it names
    /// the function rather than leaving the app to call it. The types are the
    /// same whichever language is generated, so they belong to the extension
    /// rather than to each [`ShellSource`].
    pub types: Option<RegisterTypes>,
    /// The top-level names the shipped sources declare in the generated
    /// module, in any language: `HttpHandler`, `IHttpHandler`,
    /// `URLSessionHttpHandler` and so on.
    ///
    /// The sources share a module with the app's types and the generated
    /// shell API, so when the registry is built each name is checked against
    /// the names the generated shell API reserves, the names the module
    /// imports from the serialization runtime (`Serializer`, `Bytes` and so
    /// on), the namespaces the app's types are generated into, the names other
    /// registered extensions declare, and the types registered in the root
    /// namespace. A clash is then a generation error rather than a module that
    /// fails to compile.
    pub declares: &'static [&'static str],
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
    /// They reach the manifest only for an app that registers the extension,
    /// but every one of those apps then pays for the library, so the bar is
    /// high: most extensions are [`stdlib`](Self::stdlib).
    ///
    /// A Swift package also needs the target's edge to its product, which is
    /// [`target_dependencies`](Self::target_dependencies). One caveat remains,
    /// facet-generate's: C# manifest entries are not written to the `.csproj`
    /// at all, because its C# installer does not consult plugins. An extension
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
    /// what a shipped extension should normally be.
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

impl ShellExtension {
    /// An extension that ships no language yet. A capability names it after
    /// itself: `Http`, `Time`, `KeyValue`. Add the languages it ships with the
    /// builders.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            types: None,
            declares: &[],
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

    /// The top-level names the shipped sources declare. See
    /// [`declares`](Self::declares).
    #[must_use]
    pub const fn declares(mut self, declares: &'static [&'static str]) -> Self {
        self.declares = declares;
        self
    }

    /// The Swift source this extension ships.
    #[must_use]
    pub const fn swift(mut self, source: ShellSource) -> Self {
        self.swift = Some(source);
        self
    }

    /// The Kotlin source this extension ships.
    #[must_use]
    pub const fn kotlin(mut self, source: ShellSource) -> Self {
        self.kotlin = Some(source);
        self
    }

    /// The TypeScript source this extension ships.
    #[must_use]
    pub const fn typescript(mut self, source: ShellSource) -> Self {
        self.typescript = Some(source);
        self
    }

    /// The C# source this extension ships.
    #[must_use]
    pub const fn csharp(mut self, source: ShellSource) -> Self {
        self.csharp = Some(source);
        self
    }
}
