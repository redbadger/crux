//! Shell extensions: foreign source a crate ships, emitted into the app's
//! generated module when the app registers it.
//!
//! The main use is a capability's shell handler. A capability knows how its
//! protocol works (which `URLError` is a timeout, what a cleared timer answers
//! with). Without a shipped handler, every shell finds those rules again in
//! the book. The capability can instead ship the shell side of its protocol as
//! source, versioned with the crate.
//!
//! One extension is one language. Its source is a directory, embedded with
//! [`include_dir`](https://docs.rs/include_dir), and laid out as source sets:
//!
//! ```text
//! shell/kotlin/url_connection/
//!   common/   Http.kt  UrlConnectionHttp.kt
//!   android/  WebkitCookieJar.kt
//! ```
//!
//! When the app generates code for a platform, crux emits every file in
//! `common/` and every file in the directory named after that platform. A
//! platform difference is a file in that platform's directory, not a compiler
//! directive.
//!
//! ```rust,ignore
//! // In the capability crate, in `src/shell.rs`
//! pub static URL_CONNECTION: ShellExtension = ShellExtension::kotlin(
//!     "Http",
//!     include_dir!("$CARGO_MANIFEST_DIR/shell/kotlin/url_connection"),
//! )
//! .rust_types(register_types)
//! .platforms(&[Platform::Jvm, Platform::Android])
//! .foreign_types(&["HttpHandler", "UrlConnectionHttpHandler", "WebkitCookieJar"]);
//!
//! fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
//!     HttpRequest::register_types_facet(registry)
//! }
//! ```
//!
//! Crux emits nothing for an extension that the app did not name with
//! [`TypeRegistry::shell_extension`](crate::type_generation::facet::TypeRegistry::shell_extension),
//! and no generated code calls the shipped code. For a capability's handler,
//! the app's own handler holds an instance and delegates to it, one line per
//! operation. Thus the shell makes both decisions: whether to use the shipped
//! implementation, and which platform client to use with it.

use std::{path::Path, result::Result};

pub use include_dir::Dir;

use crate::type_generation::facet::{TypeGenError, TypeRegistry};

/// How an extension registers the types its shipped sources name.
///
/// The same shape as
/// [`Export::register_types`](crate::type_generation::facet::Export::register_types)
/// and as an operation's own `register_types_facet`, so a capability with one
/// operation can name that directly.
pub type RegisterTypes = fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>;

/// The name of the directory that holds the files every platform gets.
pub(super) const COMMON: &str = "common";

/// A language in which crux generates code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    Swift,
    Kotlin,
    TypeScript,
    CSharp,
}

impl Language {
    /// The file extension of a source file in this language, without the dot.
    #[must_use]
    pub const fn file_extension(self) -> &'static str {
        match self {
            Self::Swift => "swift",
            Self::Kotlin => "kt",
            Self::TypeScript => "ts",
            Self::CSharp => "cs",
        }
    }

    /// The default platform of the language. The language's own method on
    /// [`CodeGenerator`](crate::type_generation::facet::CodeGenerator), such
    /// as `kotlin`, generates for this platform. The default platforms are
    /// Apple for Swift, the JVM for Kotlin, the web for TypeScript and .NET
    /// for C#.
    #[must_use]
    pub const fn default_platform(self) -> Platform {
        match self {
            Self::Swift => Platform::Apple,
            Self::Kotlin => Platform::Jvm,
            Self::TypeScript => Platform::Web,
            Self::CSharp => Platform::DotNet,
        }
    }

    /// The platforms the language runs on.
    #[must_use]
    pub const fn platforms(self) -> &'static [Platform] {
        match self {
            Self::Swift => &[Platform::Apple, Platform::Linux],
            Self::Kotlin => &[Platform::Jvm, Platform::Android],
            Self::TypeScript => &[Platform::Web],
            Self::CSharp => &[Platform::DotNet],
        }
    }

    /// The name of the language, for error messages.
    pub(super) const fn display_name(self) -> &'static str {
        match self {
            Self::Swift => "Swift",
            Self::Kotlin => "Kotlin",
            Self::TypeScript => "TypeScript",
            Self::CSharp => "C#",
        }
    }
}

/// A platform that a shell runs on. Each platform belongs to one
/// [`Language`].
///
/// A shell extension lists the platforms its source runs on. It can have a
/// directory of files for each of those platforms. See [`ShellExtension`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Platform {
    /// Swift on iOS, macOS, tvOS, watchOS and visionOS. The directory is
    /// `apple/`.
    Apple,
    /// Swift on Linux, where `URLSession` is in `FoundationNetworking`. The
    /// directory is `linux/`.
    Linux,
    /// Kotlin on the JVM, without the Android SDK. The directory is `jvm/`.
    Jvm,
    /// Kotlin on Android, with the Android SDK. The directory is `android/`.
    Android,
    /// TypeScript in a browser. The directory is `web/`.
    Web,
    /// C# on .NET. The directory is `dotnet/`.
    DotNet,
}

impl Platform {
    /// The language a shell on this platform is written in.
    #[must_use]
    pub const fn language(self) -> Language {
        match self {
            Self::Apple | Self::Linux => Language::Swift,
            Self::Jvm | Self::Android => Language::Kotlin,
            Self::Web => Language::TypeScript,
            Self::DotNet => Language::CSharp,
        }
    }

    /// The name of the directory that holds the files for this platform only.
    #[must_use]
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Apple => "apple",
            Self::Linux => "linux",
            Self::Jvm => "jvm",
            Self::Android => "android",
            Self::Web => "web",
            Self::DotNet => "dotnet",
        }
    }
}

/// Source a crate ships for the shell, in one language.
///
/// A capability ships its shell handler as one extension per language. A
/// language can have more than one extension when each platform needs a
/// different implementation. Each extension holds the whole capability, not
/// one operation. The operations of a capability share state (an HTTP client,
/// a key-value store, a timer table), and the capability is the unit that an
/// app configures or replaces.
///
/// The source is a directory, embedded with `include_dir!`. At its root, the
/// directory can contain only these subdirectories:
///
/// * `common/`, with the files that every platform gets.
/// * One directory for each platform of the language, named by
///   [`Platform::directory`], with the files for that platform only.
///
/// Each subdirectory contains source files only, and no further directories.
/// Crux ignores a file whose name starts with `.`, such as `.DS_Store`.
///
/// Build an extension with the constructor for its language
/// ([`swift`](Self::swift), [`kotlin`](Self::kotlin),
/// [`typescript`](Self::typescript) or [`csharp`](Self::csharp)) and the
/// `const` builders. Then a crate can declare its extension in a `static`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ShellExtension {
    /// The name of the capability, in `UpperCamelCase`: `Http`, `Time`,
    /// `KeyValue`.
    ///
    /// The extensions of one capability, in all languages, share this name.
    /// Crux registers the [`rust_types`](Self::rust_types) once for each name,
    /// and emits no more than one extension of each name for a language and
    /// platform. The name does not name a file.
    pub name: &'static str,
    /// The language of the source.
    pub language: Language,
    /// The directory of source files, laid out as source sets. See
    /// [`ShellExtension`].
    pub sources: Dir<'static>,
    /// The platforms the source runs on. Each one is a platform of
    /// [`language`](Self::language), and the list cannot be empty.
    ///
    /// The list is explicit, because a directory for a platform does not
    /// prove that the source runs there. A platform without a directory gets
    /// the files in `common/` only.
    pub platforms: &'static [Platform],
    /// Registers the Rust types the shipped sources name, run by
    /// [`TypeRegistry::shell_extension`].
    ///
    /// A capability's handler implements the whole capability, so crux must
    /// generate every operation and output, including the ones this app
    /// never sends. The capability registers them through this function, so
    /// the app does not have to know which types they are.
    pub rust_types: Option<RegisterTypes>,
    /// The names the shipped sources declare in the generated foreign code,
    /// as opposed to the Rust types in [`rust_types`](Self::rust_types). The
    /// names are the top-level names in the generated module: `HttpHandler`,
    /// `URLSessionHttpHandler` and so on.
    ///
    /// The sources share a module with the app's types and the generated
    /// shell API. When the registry is built, crux checks each name against
    /// these names:
    ///
    /// * the names the generated shell API reserves
    /// * the names the module imports from the serialization runtime
    ///   (`Serializer`, `Bytes` and so on)
    /// * the namespaces the app's types are generated into
    /// * the names that extensions of other capabilities declare
    /// * the types registered in the root namespace.
    ///
    /// Thus a clash is a generation error, not a module that fails to compile.
    pub foreign_types: &'static [&'static str],
    /// Manifest entries the source needs beyond the platform standard library,
    /// in the form facet-generate's `manifest_dependencies` takes: an SPM
    /// `.package(…)` entry in Swift, a Gradle `implementation(…)` line in
    /// Kotlin, a `"name": "version"` pair in TypeScript.
    ///
    /// The entries go into the manifest only when the app registers the
    /// extension and generates one of its platforms. Every one of those apps
    /// then pays for the library, so most extensions use only the standard
    /// library.
    ///
    /// A Swift package also needs the target's edge to its product, which is
    /// [`target_dependencies`](Self::target_dependencies).
    ///
    /// facet-generate does not write a C# extension's `dependencies` to the
    /// `.csproj`, because its C# installer reads the extension's code but not
    /// its manifest dependencies (facet-generate #134). If a C# extension needs
    /// a package, its documentation must say so.
    pub dependencies: &'static [&'static str],
    /// Edges the generated build target needs, Swift only. Crux writes each
    /// entry verbatim into the SPM target's `dependencies:` array, so each
    /// entry must be a `Target.Dependency` expression. Normally that is
    /// `.product(name: "…", package: "…")`, which names a product of a
    /// package declared in [`dependencies`](Self::dependencies).
    pub target_dependencies: &'static [&'static str],
}

impl ShellExtension {
    const fn new(name: &'static str, language: Language, sources: Dir<'static>) -> Self {
        Self {
            name,
            language,
            sources,
            platforms: &[],
            rust_types: None,
            foreign_types: &[],
            dependencies: &[],
            target_dependencies: &[],
        }
    }

    /// An extension of Swift source, for the capability called `name`.
    #[must_use]
    pub const fn swift(name: &'static str, sources: Dir<'static>) -> Self {
        Self::new(name, Language::Swift, sources)
    }

    /// An extension of Kotlin source, for the capability called `name`.
    #[must_use]
    pub const fn kotlin(name: &'static str, sources: Dir<'static>) -> Self {
        Self::new(name, Language::Kotlin, sources)
    }

    /// An extension of TypeScript source, for the capability called `name`.
    #[must_use]
    pub const fn typescript(name: &'static str, sources: Dir<'static>) -> Self {
        Self::new(name, Language::TypeScript, sources)
    }

    /// An extension of C# source, for the capability called `name`.
    #[must_use]
    pub const fn csharp(name: &'static str, sources: Dir<'static>) -> Self {
        Self::new(name, Language::CSharp, sources)
    }

    /// The platforms the source runs on. See [`platforms`](Self::platforms).
    #[must_use]
    pub const fn platforms(mut self, platforms: &'static [Platform]) -> Self {
        self.platforms = platforms;
        self
    }

    /// The function that registers the types the shipped sources name. See
    /// [`rust_types`](Self::rust_types).
    ///
    /// A plain `fn` item, or a closure that captures nothing, so that the
    /// whole declaration is still a `static`.
    #[must_use]
    pub const fn rust_types(mut self, rust_types: RegisterTypes) -> Self {
        self.rust_types = Some(rust_types);
        self
    }

    /// The top-level names the shipped sources declare. See
    /// [`foreign_types`](Self::foreign_types).
    #[must_use]
    pub const fn foreign_types(mut self, foreign_types: &'static [&'static str]) -> Self {
        self.foreign_types = foreign_types;
        self
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

    /// The files crux emits for `platform`: every file in `common/`, then
    /// every file in the platform's directory, each set sorted by path.
    /// Crux skips hidden files.
    pub(super) fn files_for(&'static self, platform: Platform) -> Vec<SourceFile> {
        let mut files = Vec::new();
        for directory in [COMMON, platform.directory()] {
            let Some(dir) = self.sources.get_dir(directory) else {
                continue;
            };
            let mut set: Vec<SourceFile> = dir
                .files()
                .filter_map(SourceFile::new)
                .filter(|file| !file.is_hidden())
                .collect();
            set.sort_by(|a, b| a.path.cmp(b.path));
            files.extend(set);
        }
        files
    }
}

/// One file of a shell extension's source.
#[derive(Debug, Clone, Copy)]
pub(super) struct SourceFile {
    /// The path relative to the extension's directory: `common/Http.kt`.
    pub path: &'static str,
    /// The file name, which is also the name of the companion file:
    /// `Http.kt`.
    pub file_name: &'static str,
    /// The file name up to its first `.`: `Http`.
    pub stem: &'static str,
    /// The file name after its first `.`: `kt`. Empty when the name has no
    /// `.`.
    pub extension: &'static str,
    /// The contents, or `None` if they are not UTF-8.
    pub contents: Option<&'static str>,
}

impl SourceFile {
    /// `None` for a path that is not UTF-8, which `include_dir!` does not
    /// produce.
    pub(super) fn new(file: &'static include_dir::File<'static>) -> Option<Self> {
        let path = file.path().to_str()?;
        let file_name = Path::new(path).file_name()?.to_str()?;
        let (stem, extension) = file_name.split_once('.').unwrap_or((file_name, ""));
        Some(Self {
            path,
            file_name,
            stem,
            extension,
            contents: file.contents_utf8(),
        })
    }

    pub(super) fn is_hidden(&self) -> bool {
        self.file_name.starts_with('.')
    }
}
