//! Emits the shell extensions the app registered, such as the handler a
//! capability ships.
//!
//! There is nothing to generate here. The extension's author writes the
//! source and the plugin emits it verbatim, where the language expects to
//! find it. The generator chooses the extensions and their files for the
//! language and platform, and the plugin receives only those files.
//!
//! Swift, Kotlin and C# get one companion file for each source file, with the
//! name of the source file (`Http.swift`, `WebkitCookieJar.kt`). The plugin
//! writes the companion file beside the module's own file, inside its module,
//! package or namespace. A TypeScript module is a single file, so the sources
//! go in the module footer, in the order of their paths. The footer comes
//! after every type, and after `Core` when `Core` is in this module.
//!
//! A namespace of its own is a module of its own in every language, so the
//! sources are emitted for the root module, which holds the app's types that
//! have no namespace, and not repeated into a namespaced sibling.
//!
//! The module header the companion-file machinery writes carries only the
//! package or namespace declaration and the generated module's own imports, so
//! a shipped source brings its own — which is also why its imports must be at
//! the top of the file, ahead of any declaration, as Kotlin requires anyway.

use std::{io, sync::Arc};

use facet_generate::{
    generation::{
        CodeGeneratorConfig,
        csharp::CSharp,
        indent::IndentWrite,
        kotlin::Kotlin,
        plugin::{CompanionFile, EmitterPlugin},
        swift::Swift,
        typescript::TypeScript,
    },
    reflection::format::Namespace,
};

use crate::type_generation::facet::shell_extension::SourceFile;

/// Whether `config` is the root module: the one named after the package,
/// which holds the app's types that have no namespace of their own.
///
/// facet-generate generates one module per namespace and calls every module's
/// plugins, and each module's config says which namespace it generates. The
/// sources belong in the root module only.
fn is_root_module(config: &CodeGeneratorConfig) -> bool {
    config.namespace == Namespace::Root
}

/// Emits the source files of the shell extensions that the generator selected
/// for one language and platform.
#[derive(Debug, Clone)]
pub struct ShellExtensionPlugin {
    /// The files to emit, in order: each selected extension in registration
    /// order, and in each extension, `common/` then the platform's directory,
    /// each sorted by path.
    files: Arc<[SourceFile]>,
    /// The union of the selected extensions' manifest entries.
    dependencies: Vec<String>,
    /// The union of the selected extensions' Swift target edges.
    target_dependencies: Vec<String>,
}

impl ShellExtensionPlugin {
    pub fn new(
        files: Vec<SourceFile>,
        dependencies: Vec<String>,
        target_dependencies: Vec<String>,
    ) -> Self {
        Self {
            files: files.into(),
            dependencies,
            target_dependencies,
        }
    }

    /// One companion file per source file, with the source file's name.
    fn companion_sources(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        if !is_root_module(config) {
            return vec![];
        }
        self.files
            .iter()
            .map(|file| CompanionFile {
                file_name: file.file_name.to_string(),
                // The shipped source carries its own imports, because only
                // the extension's author knows what it uses.
                imports: vec![],
                contents: file.contents.unwrap_or_default().to_string(),
            })
            .collect()
    }

    /// Writes the TypeScript sources into the root module's footer, in the
    /// order of their paths.
    fn append_typescript(
        &self,
        w: &mut dyn IndentWrite,
        config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        if !is_root_module(config) {
            return Ok(());
        }

        for file in self.files.iter() {
            writeln!(w)?;
            for line in file.contents.unwrap_or_default().lines() {
                writeln!(w, "{line}")?;
            }
        }

        Ok(())
    }
}

impl EmitterPlugin<Swift> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    /// A Swift package entry declares the package; the target still needs its
    /// edge to the product, which is the extension's to name.
    ///
    /// The root module's target, since that is where the sources were emitted.
    /// Every module is asked, and a namespaced one has none of them.
    fn target_dependencies(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !is_root_module(config) {
            return vec![];
        }
        self.target_dependencies.clone()
    }
}

impl EmitterPlugin<Kotlin> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}

impl EmitterPlugin<CSharp> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}

impl EmitterPlugin<TypeScript> for ShellExtensionPlugin {
    fn module_footer(
        &self,
        w: &mut dyn IndentWrite,
        config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        self.append_typescript(w, config)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}
