//! Emits the protocols of the capabilities that the app uses.
//!
//! There is nothing to generate here. The capability's author writes the
//! source and the plugin emits it unchanged, where the language expects to
//! find it. Swift, Kotlin and C# get one companion file for each protocol,
//! with the name of the protocol (`Http.swift`, `Http.kt`, `Http.cs`). The
//! plugin writes the companion file beside the module's own file, inside its
//! module, package or namespace. A TypeScript module is a single file, so the
//! sources go in the module footer, in registration order, after every type.
//!
//! A namespace of its own is a module of its own in every language. Thus the
//! plugin emits the sources only for the root module, which holds the app's
//! types that have no namespace. It does not repeat them into a namespaced
//! module.
//!
//! The module header that the companion-file machinery writes holds only the
//! package or namespace declaration and the generated module's own imports.
//! Thus a protocol source brings its own imports, at the top of the file.

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

use crate::type_generation::facet::shell_protocol::{ProtocolSource, ShellProtocol};

/// Emits the sources of the registered protocols.
#[derive(Debug, Clone)]
pub struct ShellProtocolPlugin {
    /// The protocols to emit, in registration order.
    protocols: Arc<[&'static ShellProtocol]>,
}

impl ShellProtocolPlugin {
    pub fn new(protocols: &[&'static ShellProtocol]) -> Self {
        Self {
            protocols: protocols.into(),
        }
    }

    /// Whether `config` is the root module: the module named after the
    /// package, which holds the app's types that have no namespace of their
    /// own.
    ///
    /// facet-generate generates one module for each namespace and calls the
    /// plugins of every module. The config of each module tells which
    /// namespace it generates.
    fn is_root_module(config: &CodeGeneratorConfig) -> bool {
        config.namespace == Namespace::Root
    }

    /// One companion file for each protocol that has source in this language,
    /// with the name of the protocol.
    fn companion_sources(
        &self,
        config: &CodeGeneratorConfig,
        extension: &str,
        source_of: fn(&ShellProtocol) -> Option<&ProtocolSource>,
    ) -> Vec<CompanionFile> {
        if !Self::is_root_module(config) {
            return vec![];
        }
        self.protocols
            .iter()
            .filter_map(|protocol| {
                source_of(protocol).map(|shipped| CompanionFile {
                    file_name: format!("{}.{extension}", protocol.name),
                    // The source carries its own imports, because only the
                    // author of the capability knows what it uses.
                    imports: vec![],
                    contents: shipped.source.to_string(),
                })
            })
            .collect()
    }

    /// Writes the TypeScript sources into the footer of the root module.
    fn append_typescript(
        &self,
        w: &mut dyn IndentWrite,
        config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        if !Self::is_root_module(config) {
            return Ok(());
        }

        for shipped in self.protocols.iter().filter_map(|p| p.typescript.as_ref()) {
            writeln!(w)?;
            for line in shipped.source.lines() {
                writeln!(w, "{line}")?;
            }
        }

        Ok(())
    }
}

impl EmitterPlugin<Swift> for ShellProtocolPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "swift", |protocol| protocol.swift.as_ref())
    }
}

impl EmitterPlugin<Kotlin> for ShellProtocolPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "kt", |protocol| protocol.kotlin.as_ref())
    }
}

impl EmitterPlugin<CSharp> for ShellProtocolPlugin {
    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        self.companion_sources(config, "cs", |protocol| protocol.csharp.as_ref())
    }
}

impl EmitterPlugin<TypeScript> for ShellProtocolPlugin {
    fn module_footer(
        &self,
        w: &mut dyn IndentWrite,
        config: &CodeGeneratorConfig,
    ) -> io::Result<()> {
        self.append_typescript(w, config)
    }
}
