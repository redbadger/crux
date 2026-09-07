//! Emits the effect handler API: a sink for streams, a handler protocol /
//! interface the shell implements, and a dispatcher that resolves requests.
//!
//! The point of the generated dispatcher is that a shell never calls `resolve`
//! itself, so it cannot resolve the wrong number of times or with bytes of the
//! wrong type. A notification is not resolved at all, a request is resolved
//! once with the value the handler returned, and each item a stream's sink
//! receives is one resolution.
//!
//! Operations that declare no kind keep the old shape: the handler method is
//! handed the request id and a `resolve` callback taking raw bytes.

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
use crate::type_generation::facet::EffectMeta;

/// Emits `EffectSink`, `EffectHandler` and `EffectDispatcher`.
#[derive(Debug, Clone)]
pub struct EffectHandlerPlugin {
    effects: Arc<[EffectMeta]>,
}

impl EffectHandlerPlugin {
    pub fn new(effects: &Arc<[EffectMeta]>) -> Self {
        Self {
            effects: Arc::clone(effects),
        }
    }

    /// The handler API uses fixed names, so it is emitted for the first
    /// registered effect only.
    fn matched<'a>(&'a self, ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
        matched(&self.effects, ctx).filter(|m| m.primary)
    }

    /// Whether the handler has an asynchronous method, which is what needs
    /// `Task` imported in C#.
    fn has_request(&self) -> bool {
        self.effects.first().is_some_and(|effect| {
            effect
                .variants
                .iter()
                .any(|variant| variant.kind == Some(crate::RequestKind::Request))
        })
    }
}

impl EmitterPlugin<Swift> for EffectHandlerPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| swift::emit(w, &m, ctx.config))
    }
}

impl EmitterPlugin<Kotlin> for EffectHandlerPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| kotlin::emit(w, &m, ctx.config))
    }
}

impl EmitterPlugin<TypeScript> for EffectHandlerPlugin {
    fn imports(&self, config: &CodeGeneratorConfig) -> Vec<String> {
        if !serializes_output(&self.effects) {
            return vec![];
        }
        let path = bincode_import_path(config);
        vec![format!(r#"import {{ BincodeSerializer }} from "{path}";"#)]
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| typescript::emit(w, &m, ctx.config))
    }
}

impl EmitterPlugin<CSharp> for EffectHandlerPlugin {
    fn imports(&self, _config: &CodeGeneratorConfig) -> Vec<String> {
        if self.effects.is_empty() {
            return vec![];
        }
        let mut imports = vec!["using System;".to_string()];
        if self.has_request() {
            imports.push("using System.Threading.Tasks;".to_string());
        }
        imports
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| csharp::emit(w, &m, ctx.config))
    }
}
