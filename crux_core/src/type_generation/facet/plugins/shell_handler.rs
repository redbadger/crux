//! Emits the shell handlers a capability ships, for the app that asked for
//! them.
//!
//! There is nothing to generate here — the source is written by the capability
//! author and emitted verbatim — so the plugin's whole job is putting it where
//! the language expects to find it. Swift, Kotlin and C# get one companion
//! file per handler, `Http.swift` / `Http.kt` / `Http.cs`, beside the module's
//! own file and inside its module, package or namespace. TypeScript's modules
//! are a single file, so the source is appended after the types, the way
//! `Core` is.
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
    shell_handler::{ShellHandler, ShellSource},
};

/// Emits the registered shell handlers' sources.
#[derive(Debug, Clone)]
pub struct ShellHandlerPlugin {
    handlers: Arc<[&'static ShellHandler]>,
    /// The module the app's own types are in, which is the one the sources
    /// belong to. A type in a namespace is generated into a module of its own,
    /// and every module's plugins are asked for their files.
    package: String,
    /// The last type that module writes, which is what TypeScript appends
    /// after. `None` when nothing was registered, in which case there is no
    /// `after_type` to hook and the TypeScript sources are not emitted.
    last_type: Option<QualifiedTypeName>,
}

impl ShellHandlerPlugin {
    pub fn new(
        handlers: &[&'static ShellHandler],
        package: &str,
        last_type: Option<QualifiedTypeName>,
    ) -> Self {
        Self {
            handlers: handlers.into(),
            package: package.to_string(),
            last_type,
        }
    }

    /// Whether this is the module the app's types are in, rather than one
    /// generated for a namespace of theirs.
    fn is_app_module(&self, config: &CodeGeneratorConfig) -> bool {
        config.module_name() == self.package
    }

    /// One companion file per handler that ships this language, named after
    /// the handler.
    fn companion_sources(
        &self,
        config: &CodeGeneratorConfig,
        extension: &str,
        source_of: fn(&ShellHandler) -> Option<&ShellSource>,
    ) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
            return vec![];
        }
        self.handlers
            .iter()
            .filter_map(|handler| {
                source_of(handler).map(|shipped| CompanionFile {
                    file_name: format!("{}.{extension}", handler.name),
                    // The shipped source carries its own imports: only the
                    // capability author knows what it uses.
                    imports: vec![],
                    contents: shipped.source.to_string(),
                })
            })
            .collect()
    }

    /// The union of one list of the registered handlers' sources, in
    /// registration order and without repeats.
    fn union(
        &self,
        source_of: fn(&ShellHandler) -> Option<&ShellSource>,
        entries_of: fn(&ShellSource) -> &'static [&'static str],
    ) -> Vec<String> {
        let mut entries: Vec<String> = Vec::new();
        for shipped in self.handlers.iter().filter_map(|h| source_of(h)) {
            for entry in entries_of(shipped) {
                let entry = (*entry).to_string();
                if !entries.contains(&entry) {
                    entries.push(entry);
                }
            }
        }
        entries
    }

    /// What the registered handlers' sources need in the manifest.
    fn dependencies(&self, source_of: fn(&ShellHandler) -> Option<&ShellSource>) -> Vec<String> {
        self.union(source_of, |shipped| shipped.dependencies)
    }

    /// Writes the TypeScript sources after the module's last type, so that
    /// every type they name is already declared above them.
    fn append_typescript(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        if !self.is_app_module(ctx.config) || self.last_type.as_ref() != Some(ctx.container.name) {
            return Ok(());
        }

        for shipped in self.handlers.iter().filter_map(|h| h.typescript.as_ref()) {
            writeln!(w)?;
            for line in shipped.source.lines() {
                writeln!(w, "{line}")?;
            }
        }

        Ok(())
    }
}

impl EmitterPlugin<Swift> for ShellHandlerPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "swift", |handler| handler.swift.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|handler| handler.swift.as_ref())
    }

    /// A Swift package entry declares the package; the target still needs its
    /// edge to the product, which is the handler's to name.
    fn target_dependencies(&self) -> Vec<String> {
        self.union(
            |handler| handler.swift.as_ref(),
            |shipped| shipped.target_dependencies,
        )
    }
}

impl EmitterPlugin<Kotlin> for ShellHandlerPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "kt", |handler| handler.kotlin.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|handler| handler.kotlin.as_ref())
    }
}

impl EmitterPlugin<CSharp> for ShellHandlerPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "cs", |handler| handler.csharp.as_ref())
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|handler| handler.csharp.as_ref())
    }
}

impl EmitterPlugin<TypeScript> for ShellHandlerPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.append_typescript(w, ctx)
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.dependencies(|handler| handler.typescript.as_ref())
    }
}
