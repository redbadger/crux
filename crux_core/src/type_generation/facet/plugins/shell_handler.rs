//! Emits the shell extensions the app registered, such as the handler a
//! capability ships.
//!
//! There is nothing to generate here. The source is written by the
//! extension's author and emitted verbatim, so the plugin's whole job is
//! putting it where the language expects to find it. Swift, Kotlin and C# get
//! one companion file per extension, `Http.swift` / `Http.kt` / `Http.cs`,
//! beside the module's own file and inside its module, package or namespace.
//! TypeScript's modules are a single file, so the source is appended after the
//! types, the way `Core` is.
//!
//! A namespace of its own is a module of its own in every language, so the
//! sources are emitted for the module the app's own types are in — named by
//! the package — and not repeated into a namespaced sibling.
//!
//! The module header the companion-file machinery writes carries only the
//! package or namespace declaration and the generated module's own imports, so
//! a shipped source brings its own — which is also why its imports must be at
//! the top of the file, ahead of any declaration, as Kotlin requires anyway.

use std::{io, sync::Arc};

use facet_generate::generation::{
    CodeGeneratorConfig,
    csharp::CSharp,
    indent::IndentWrite,
    kotlin::Kotlin,
    plugin::{CompanionFile, EmitContext, EmitterPlugin},
    swift::Swift,
    typescript::TypeScript,
};

use crate::type_generation::facet::{
    QualifiedTypeName,
    shell_handler::{ShellExtension, ShellSource},
};

/// Emits the registered shell extensions' sources.
#[derive(Debug, Clone)]
pub struct ShellExtensionPlugin {
    extensions: Arc<[&'static ShellExtension]>,
    /// The module the app's own types are in, which is the one the sources
    /// belong to. A type in a namespace is generated into a module of its own,
    /// and every module's plugins are asked for their files.
    package: String,
    /// The last type that module writes, which is what TypeScript appends
    /// after. `None` when nothing was registered, in which case there is no
    /// `after_type` to hook and the TypeScript sources are not emitted.
    ///
    /// Plugin order cannot do this instead: it orders the plugins within one
    /// hook, but it does not choose which type an `after_type` call follows.
    /// TypeScript has no companion files, so the source has to go into the
    /// module's one file, after its last type. Writing it ahead of the types
    /// would be unsafe in general, because an extension's top-level code may
    /// construct or extend a generated class, and a class cannot be used until
    /// its declaration has run.
    last_type: Option<QualifiedTypeName>,
}

impl ShellExtensionPlugin {
    pub fn new(
        extensions: &[&'static ShellExtension],
        package: &str,
        last_type: Option<QualifiedTypeName>,
    ) -> Self {
        Self {
            extensions: extensions.into(),
            package: package.to_string(),
            last_type,
        }
    }

    /// Whether this is the module the app's root types are in.
    ///
    /// facet-generate generates one module per facet namespace, and asks the
    /// plugins of every module for their companion files. The sources belong
    /// only in the module named by the package, which holds the app's types
    /// that have no namespace of their own.
    fn is_app_module(&self, config: &CodeGeneratorConfig) -> bool {
        config.module_name() == self.package
    }

    /// One companion file per extension that ships this language, named after
    /// the extension.
    fn companion_sources(
        &self,
        config: &CodeGeneratorConfig,
        extension: &str,
        source_of: fn(&ShellExtension) -> Option<&ShellSource>,
    ) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
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

    /// Writes the TypeScript sources after the module's last type, so that
    /// every type they name is already declared above them.
    ///
    /// TypeScript is the one language whose module is a single file, with no
    /// companion files beside it, so its sources are written into the module
    /// itself rather than returned from `companion_files`.
    fn append_typescript(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        if !self.is_app_module(ctx.config) || self.last_type.as_ref() != Some(ctx.container.name) {
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
    fn target_dependencies(&self, _config: &CodeGeneratorConfig) -> Vec<String> {
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
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.append_typescript(w, ctx)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|extension| extension.typescript.as_ref())
    }
}
