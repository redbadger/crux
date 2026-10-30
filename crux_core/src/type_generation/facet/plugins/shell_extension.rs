//! Emits the shell extensions the app registered, such as the handler a
//! capability ships.
//!
//! There is nothing to generate here. The source is written by the
//! extension's author and emitted verbatim, so the plugin's whole job is
//! putting it where the language expects to find it. Swift, Kotlin and C# get
//! one companion file per extension, `Http.swift` / `Http.kt` / `Http.cs`,
//! beside the module's own file and inside its module, package or namespace.
//! TypeScript modules are one file, so the source goes in the module footer,
//! after every type (and after `Core`, when it is in this module).
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

use crate::type_generation::facet::shell_extension::{ShellExtension, ShellSource};

/// Whether `config` is the root module: the one named after the package,
/// which holds the app's types that have no namespace of their own.
///
/// facet-generate generates one module per namespace and calls every module's
/// plugins, and each module's config says which namespace it generates. The
/// sources belong in the root module only.
fn is_root_module(config: &CodeGeneratorConfig) -> bool {
    config.namespace == Namespace::Root
}

/// Emits the registered shell extensions' sources.
#[derive(Debug, Clone)]
pub struct ShellExtensionPlugin {
    extensions: Arc<[&'static ShellExtension]>,
}

impl ShellExtensionPlugin {
    pub fn new(extensions: &[&'static ShellExtension]) -> Self {
        Self {
            extensions: extensions.into(),
        }
    }

    /// One companion file per extension that ships this language, named after
    /// the extension.
    fn companion_sources(
        &self,
        config: &CodeGeneratorConfig,
        extension: &str,
        source_of: fn(&ShellExtension) -> Option<&ShellSource>,
    ) -> Vec<CompanionFile> {
        if !is_root_module(config) {
            return vec![];
        }
        self.extensions
            .iter()
            .filter_map(|shell_extension| {
                source_of(shell_extension).map(|shipped| CompanionFile {
                    file_name: format!("{}.{extension}", shell_extension.name),
                    // The shipped source carries its own imports: only the
                    // extension's author knows what it uses.
                    imports: vec![],
                    contents: shipped.source.to_string(),
                })
            })
            .collect()
    }

    /// The union of one list of the registered extensions' sources, in
    /// registration order and without repeats.
    fn union(
        &self,
        source_of: fn(&ShellExtension) -> Option<&ShellSource>,
        entries_of: fn(&ShellSource) -> &'static [&'static str],
    ) -> Vec<String> {
        let mut entries: Vec<String> = Vec::new();
        for shipped in self.extensions.iter().filter_map(|e| source_of(e)) {
            for entry in entries_of(shipped) {
                let entry = (*entry).to_string();
                if !entries.contains(&entry) {
                    entries.push(entry);
                }
            }
        }
        entries
    }

    /// What the registered extensions' sources need in the manifest.
    fn dependencies(&self, source_of: fn(&ShellExtension) -> Option<&ShellSource>) -> Vec<String> {
        self.union(source_of, |shipped| shipped.dependencies)
    }

    /// Writes the TypeScript sources into the root module's footer.
    fn append_typescript(
        &self,
        w: &mut dyn IndentWrite,
        config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        if !is_root_module(config) {
            return Ok(());
        }

        for shipped in self.extensions.iter().filter_map(|e| e.typescript.as_ref()) {
            writeln!(w)?;
            for line in shipped.source.lines() {
                writeln!(w, "{line}")?;
            }
        }

        Ok(())
    }
}

impl EmitterPlugin<Swift> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "swift", |extension| extension.swift.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|extension| extension.swift.as_ref())
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
        self.union(
            |extension| extension.swift.as_ref(),
            |shipped| shipped.target_dependencies,
        )
    }
}

impl EmitterPlugin<Kotlin> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "kt", |extension| extension.kotlin.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|extension| extension.kotlin.as_ref())
    }
}

impl EmitterPlugin<CSharp> for ShellExtensionPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "cs", |extension| extension.csharp.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|extension| extension.csharp.as_ref())
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
        self.dependencies(|extension| extension.typescript.as_ref())
    }
}
