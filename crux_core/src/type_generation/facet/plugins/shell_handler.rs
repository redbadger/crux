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
//! package or namespace. A TypeScript module is a single file, so the plugin
//! appends the sources after the types, as it does for `Core`, in the order of
//! their paths.
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

use crate::type_generation::facet::{QualifiedTypeName, shell_handler::SourceFile};

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
        files: Vec<SourceFile>,
        dependencies: Vec<String>,
        target_dependencies: Vec<String>,
        package: &str,
        last_type: Option<QualifiedTypeName>,
    ) -> Self {
        Self {
            files: files.into(),
            dependencies,
            target_dependencies,
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

    /// One companion file per source file, with the source file's name.
    fn companion_sources(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
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
    /// The app's target, since that is where the sources were emitted — every
    /// module is asked, and a namespaced one has none of them.
    fn target_dependencies(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.is_app_module(config) {
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
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.append_typescript(w, ctx)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }
}
