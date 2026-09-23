//! Emits the shell's side of the core loop: a byte-level `CoreBridge` the
//! shell implements around its FFI bindings, and a `Core` that drives the loop
//! through it.
//!
//! The loop is the same in every shell and every language — serialize the
//! event, cross the FFI, deserialize the requests, dispatch each one, resolve
//! with what comes back, and go round again — so it is generated rather than
//! written out once per app.
//!
//! `Core` owns the render variant: it intercepts it before dispatching,
//! re-reads the view from the bridge and notifies. That is why it is emitted
//! only for an effect that has one, and why the handler's `render` method has a
//! default that does nothing.
//!
//! When the shell tells type generation where `BoltFFI` put its bindings (see
//! [`BoltFfi`]), the adapter between the two is generated as well: an
//! `FfiBridge` implementing `CoreBridge` over `CoreFfi`, and a one-argument
//! `Core` constructor that builds one. `CoreBridge` stays, because it is the
//! seam a fake or a middleware stack plugs into.

mod csharp;
mod kotlin;
mod swift;
mod typescript;

use std::{io, sync::Arc};

use facet_generate::generation::{
    CodeGeneratorConfig,
    csharp::CSharp,
    indent::{IndentWrite, IndentedWriter},
    kotlin::Kotlin,
    plugin::{CompanionFile, EmitContext, EmitterPlugin},
    swift::Swift,
    typescript::TypeScript,
};

use super::{Matched, Requalify, bincode_import_path, matched, serializes_output};
use crate::type_generation::facet::{AppMeta, BoltFfi, EffectMeta};

/// The version of `kotlinx-coroutines-core` the generated Kotlin package asks
/// for. `Core` uses `CoroutineScope`, `launch` and `StateFlow`, none of which
/// are in the standard library.
const KOTLIN_COROUTINES_VERSION: &str = "1.10.2";

/// Emits `CoreBridge` and `Core`, and — when the FFI bindings are named —
/// `FfiBridge`.
#[derive(Debug, Clone)]
pub struct CorePlugin {
    effects: Arc<[EffectMeta]>,
    app: AppMeta,
    ffi: Option<BoltFfi>,
    /// The module the app's own types are in, which is the only one any of
    /// this belongs to. A type in a namespace is generated into a module of
    /// its own, and every module's plugins are asked for their imports and
    /// companion files.
    package: String,
}

impl CorePlugin {
    pub fn new(
        effects: &Arc<[EffectMeta]>,
        app: AppMeta,
        ffi: Option<BoltFfi>,
        package: &str,
    ) -> Self {
        Self {
            effects: Arc::clone(effects),
            app,
            ffi,
            package: package.to_string(),
        }
    }

    /// Whether this is the module the app's types are in, rather than one
    /// generated for a namespace of theirs.
    ///
    /// The `Core` itself lands in the right place without asking, because
    /// `after_type` only fires for the effect enum — but the module-level
    /// hooks are called for every module, and a namespaced one has no `Core`
    /// for an `FfiBridge` to bridge to. Emitting one there is not merely
    /// untidy: the companion file names `CoreBridge`, which is declared in
    /// this module alone, so it does not compile.
    fn is_app_module(&self, config: &CodeGeneratorConfig) -> bool {
        config.module_name() == self.package
    }

    /// `Core` uses fixed names and owns the view loop, so it is emitted for the
    /// first registered effect, and only if that effect has a render variant.
    fn matched<'a, L: Requalify>(&'a self, ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
        matched::<L>(&self.effects, ctx).filter(|m| m.primary && m.render_variant().is_some())
    }

    /// The app's event and view model in `L`'s spelling. [`AppMeta`] records
    /// them as the registry names them, so this is the one place they are
    /// respelled.
    fn app<L: Requalify>(&self, config: &CodeGeneratorConfig) -> AppMeta {
        AppMeta {
            event: L::requalify(config, &self.app.event),
            view_model: L::requalify(config, &self.app.view_model),
        }
    }

    /// The same question as [`matched`](Self::matched), for the hooks that are
    /// called once per module and get no container to match against.
    fn emits_core(&self) -> bool {
        self.effects
            .first()
            .is_some_and(|effect| effect.variants.iter().any(|variant| variant.render))
    }

    /// The bridge configuration, if there is a `Core` for a bridge to belong
    /// to.
    fn ffi(&self) -> Option<&BoltFfi> {
        self.ffi.as_ref().filter(|_| self.emits_core())
    }
}

/// Render a companion file's body into a string, indented the way the module
/// it sits beside is.
fn render(
    config: &CodeGeneratorConfig,
    write: impl FnOnce(&mut dyn IndentWrite) -> io::Result<()>,
) -> io::Result<String> {
    let mut buffer = Vec::new();
    {
        let mut w = IndentedWriter::new(&mut buffer, config.indent);
        write(&mut w)?;
    }
    String::from_utf8(buffer).map_err(io::Error::other)
}

impl EmitterPlugin<Swift> for CorePlugin {
    /// `Core` is `@Observable`, which lives in the `Observation` framework and
    /// is not implicitly available.
    ///
    /// The FFI module is deliberately *not* imported here: only `FfiBridge`
    /// names it, and that lives in its own file.
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() || !self.is_app_module(config) {
            return vec![];
        }
        vec!["Observation".to_string()]
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched::<Swift>(ctx).map_or(Ok(()), |m| {
            swift::emit(
                w,
                &m,
                &self.app::<Swift>(ctx.config),
                ctx.config,
                self.ffi(),
            )
        })
    }

    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
            return vec![];
        }
        let Some(ffi) = self.ffi() else {
            return vec![];
        };
        let Some(swift) = ffi.swift_ffi() else {
            return vec![];
        };

        swift::ffi_bridge(swift, ffi.class_name(), config).map_or_else(
            |_| vec![],
            |contents| {
                vec![CompanionFile {
                    file_name: "FfiBridge.swift".to_string(),
                    // `Data` is Foundation's; `CoreFfi` is the FFI module's.
                    imports: vec!["Foundation".to_string(), swift.module.clone()],
                    contents,
                }]
            },
        )
    }

    /// The generated package has to depend on the package the bindings are in.
    fn manifest_dependencies(&self) -> Vec<String> {
        self.ffi()
            .and_then(BoltFfi::swift_ffi)
            .map(|swift| vec![format!(r#".package(path: "{}")"#, swift.package_path)])
            .unwrap_or_default()
    }

    /// …and the generated target has to depend on its product — the app's
    /// target, that is, which is the only one that names the bridge.
    fn target_dependencies(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.is_app_module(config) {
            return vec![];
        }
        self.ffi()
            .and_then(BoltFfi::swift_ffi)
            .map(|swift| {
                vec![format!(
                    r#".product(name: "{}", package: "{}")"#,
                    swift.product,
                    swift::package_name(&swift.package_path)
                )]
            })
            .unwrap_or_default()
    }
}

impl EmitterPlugin<Kotlin> for CorePlugin {
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() || !self.is_app_module(config) {
            return vec![];
        }
        [
            "import kotlinx.coroutines.CoroutineScope",
            "import kotlinx.coroutines.launch",
            "import kotlinx.coroutines.flow.MutableStateFlow",
            "import kotlinx.coroutines.flow.StateFlow",
            "import kotlinx.coroutines.flow.asStateFlow",
        ]
        .iter()
        .map(ToString::to_string)
        .collect()
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        if !self.emits_core() {
            return vec![];
        }
        vec![format!(
            r#"    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:{KOTLIN_COROUTINES_VERSION}")"#
        )]
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched::<Kotlin>(ctx).map_or(Ok(()), |m| {
            kotlin::emit(
                w,
                &m,
                &self.app::<Kotlin>(ctx.config),
                ctx.config,
                self.ffi(),
            )
        })
    }

    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
            return vec![];
        }
        let Some(ffi) = self.ffi() else {
            return vec![];
        };
        let Some(kotlin) = ffi.kotlin_ffi() else {
            return vec![];
        };

        kotlin::ffi_bridge(ffi.class_name(), config).map_or_else(
            |_| vec![],
            |contents| {
                // `CoreFfi` resolves without an import when BoltFFI generated
                // it into the package the types are generated into.
                let imports = kotlin
                    .package
                    .iter()
                    .filter(|package| *package != &config.module_name)
                    .map(|package| format!("import {package}.{}", ffi.class_name()))
                    .collect();

                vec![CompanionFile {
                    file_name: "FfiBridge.kt".to_string(),
                    imports,
                    contents,
                }]
            },
        )
    }
}

impl EmitterPlugin<TypeScript> for CorePlugin {
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() || !self.is_app_module(config) {
            return vec![];
        }
        let path = bincode_import_path(config);
        // TypeScript imports are written verbatim, so only ask for the
        // serializer if the handler plugin has not already imported it.
        let mut imports = vec![format!(
            r#"import {{ BincodeDeserializer }} from "{path}";"#
        )];
        if !serializes_output(&self.effects) {
            imports.push(format!(r#"import {{ BincodeSerializer }} from "{path}";"#));
        }
        // The whole namespace, because the module's `initialized` promise is
        // not in its type declarations.
        if let Some(ts) = self.ffi().and_then(BoltFfi::typescript_ffi) {
            imports.push(format!(
                r#"import * as boltffi from "{}";"#,
                ts.package.clone()
            ));
        }
        imports
    }

    /// TypeScript generates one file per module, so `FfiBridge` goes in beside
    /// `Core` rather than into a companion file.
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched::<TypeScript>(ctx).map_or(Ok(()), |m| {
            typescript::emit(
                w,
                &m,
                &self.app::<TypeScript>(ctx.config),
                ctx.config,
                self.ffi(),
            )
        })
    }

    fn manifest_dependencies(&self) -> Vec<String> {
        self.ffi()
            .and_then(BoltFfi::typescript_ffi)
            .map(|ts| vec![ts.manifest_dependency()])
            .unwrap_or_default()
    }
}

impl EmitterPlugin<CSharp> for CorePlugin {
    /// `Core` raises `PropertyChanged`, so it needs the interface, the delegate
    /// and the event args.
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() || !self.is_app_module(config) {
            return vec![];
        }
        vec!["using System.ComponentModel;".to_string()]
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched::<CSharp>(ctx).map_or(Ok(()), |m| {
            csharp::emit(
                w,
                &m,
                &self.app::<CSharp>(ctx.config),
                ctx.config,
                self.ffi(),
            )
        })
    }

    fn companion_files(&self, config: &CodeGeneratorConfig) -> Vec<CompanionFile> {
        if !self.is_app_module(config) {
            return vec![];
        }
        let Some(ffi) = self.ffi() else {
            return vec![];
        };
        let Some(csharp) = ffi.csharp_ffi() else {
            return vec![];
        };

        csharp::ffi_bridge(ffi.class_name(), config).map_or_else(
            |_| vec![],
            |contents| {
                // The module header already declares the generated namespace,
                // so a `using` is needed only for another one.
                let imports = csharp
                    .namespace
                    .iter()
                    .filter(|ns| *ns != &config.module_name)
                    .map(|ns| format!("using {ns};"))
                    .collect();

                vec![CompanionFile {
                    file_name: "FfiBridge.cs".to_string(),
                    imports,
                    contents,
                }]
            },
        )
    }
}
