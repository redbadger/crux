//! Emits the `RequestKind` type and the accessor that reports which kind an
//! effect is.
//!
//! The kind is static per operation, so it costs nothing on the wire: the
//! shell reads it off the effect it already has, and knows whether to resolve
//! the request never, once, or many times.

mod csharp;
mod kotlin;
mod swift;
mod typescript;

use std::{io, sync::Arc};

use facet_generate::generation::{
    csharp::CSharp,
    indent::IndentWrite,
    kotlin::Kotlin,
    plugin::{EmitContext, EmitterPlugin},
    swift::Swift,
    typescript::TypeScript,
};

use super::{Matched, matched};
use crate::type_generation::facet::EffectMeta;

/// Emits `RequestKind` and the per-effect accessor.
#[derive(Debug, Clone)]
pub struct RequestKindPlugin {
    effects: Arc<[EffectMeta]>,
}

impl RequestKindPlugin {
    pub fn new(effects: &Arc<[EffectMeta]>) -> Self {
        Self {
            effects: Arc::clone(effects),
        }
    }

    fn matched<'a>(&'a self, ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
        matched(&self.effects, ctx)
    }
}

impl EmitterPlugin<Swift> for RequestKindPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx).map_or(Ok(()), |m| swift::emit(w, &m))
    }
}

impl EmitterPlugin<Kotlin> for RequestKindPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| kotlin::emit(w, &m, ctx))
    }
}

impl EmitterPlugin<TypeScript> for RequestKindPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| typescript::emit(w, &m, ctx))
    }
}

impl EmitterPlugin<CSharp> for RequestKindPlugin {
    /// C# gets the accessor as a property on the `Effect` record itself. The
    /// emitter does not declare that record `partial`, so a property cannot be
    /// added from outside the way Swift and Kotlin extensions do.
    fn type_body(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| csharp::emit_accessor(w, &m, ctx))
    }

    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| csharp::emit_enum(w, &m))
    }
}
