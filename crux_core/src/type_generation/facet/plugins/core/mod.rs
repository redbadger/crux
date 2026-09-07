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

mod csharp;
mod kotlin;
mod swift;
mod typescript;

use std::{io, sync::Arc};

use facet_generate::generation::{
    CodeGeneratorConfig,
    csharp::CSharp,
    indent::IndentWrite,
    kotlin::Kotlin,
    plugin::{EmitContext, EmitterPlugin},
    swift::Swift,
    typescript::TypeScript,
};

use super::{Matched, bincode_import_path, matched, serializes_output};
use crate::type_generation::facet::{AppMeta, EffectMeta};

/// The version of `kotlinx-coroutines-core` the generated Kotlin package asks
/// for. `Core` uses `CoroutineScope`, `launch` and `StateFlow`, none of which
/// are in the standard library.
const KOTLIN_COROUTINES_VERSION: &str = "1.10.2";

/// Emits `CoreBridge` and `Core`.
#[derive(Debug, Clone)]
pub struct CorePlugin {
    effects: Arc<[EffectMeta]>,
    app: AppMeta,
}

impl CorePlugin {
    pub fn new(effects: &Arc<[EffectMeta]>, app: AppMeta) -> Self {
        Self {
            effects: Arc::clone(effects),
            app,
        }
    }

    /// `Core` uses fixed names and owns the view loop, so it is emitted for the
    /// first registered effect, and only if that effect has a render variant.
    fn matched<'a>(&'a self, ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
        matched(&self.effects, ctx).filter(|m| m.primary && m.render_variant().is_some())
    }

    /// The same question as [`matched`](Self::matched), for the hooks that are
    /// called once per module and get no container to match against.
    fn emits_core(&self) -> bool {
        self.effects
            .first()
            .is_some_and(|effect| effect.variants.iter().any(|variant| variant.render))
    }
}

impl EmitterPlugin<Swift> for CorePlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| swift::emit(w, &m, &self.app, ctx.config))
    }
}

impl EmitterPlugin<Kotlin> for CorePlugin {
    fn imports(&self, _config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() {
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
        self.matched(ctx)
            .map_or(Ok(()), |m| kotlin::emit(w, &m, &self.app, ctx.config))
    }
}

impl EmitterPlugin<TypeScript> for CorePlugin {
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !self.emits_core() {
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
        imports
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| typescript::emit(w, &m, &self.app, ctx.config))
    }
}

impl EmitterPlugin<CSharp> for CorePlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| csharp::emit(w, &m, &self.app, ctx.config))
    }
}
