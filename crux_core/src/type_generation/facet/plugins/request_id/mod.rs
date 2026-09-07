//! Emits `EffectKind` and the `RequestId` decoder — the shell's way of reading
//! a request id.
//!
//! An id the core issues is structured: it names the effect the request
//! carries, whether the shell resolves it once or many times, and an ascending
//! sequence number. The layout is Crux's, so the decoder is generated rather
//! than written by hand in every shell — and it is generated from the same
//! effect metadata the id is built from, so the two cannot drift apart.
//!
//! Nothing here changes how a request is resolved: the id still goes back to
//! the core exactly as it arrived. It is for logging, tracing and assertions —
//! saying *which* effect a stray resolve belonged to.

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

/// The bit the request id sets for a stream and clears for a request. Must
/// agree with `EffectId` in `crux_core::bridge`.
pub(super) const STREAM_BIT: u32 = 1 << SEQUENCE_BITS;

/// How many low bits of the request id hold the sequence number.
pub(super) const SEQUENCE_BITS: u32 = 23;

/// The mask that picks the sequence out of a request id.
pub(super) const SEQUENCE_MASK: u32 = STREAM_BIT - 1;

/// How far the effect's variant index is shifted up in the request id.
pub(super) const EFFECT_SHIFT: u32 = SEQUENCE_BITS + 1;

/// Emits `EffectKind` and `RequestId`.
#[derive(Debug, Clone)]
pub struct RequestIdPlugin {
    effects: Arc<[EffectMeta]>,
}

impl RequestIdPlugin {
    pub fn new(effects: &Arc<[EffectMeta]>) -> Self {
        Self {
            effects: Arc::clone(effects),
        }
    }

    /// `EffectKind` and `RequestId` use fixed names, so they are emitted for
    /// the first registered effect only.
    fn matched<'a>(&'a self, ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
        matched(&self.effects, ctx).filter(|m| m.primary)
    }
}

impl EmitterPlugin<Swift> for RequestIdPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx).map_or(Ok(()), |m| swift::emit(w, &m))
    }
}

impl EmitterPlugin<Kotlin> for RequestIdPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx).map_or(Ok(()), |m| kotlin::emit(w, &m))
    }
}

impl EmitterPlugin<TypeScript> for RequestIdPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| typescript::emit(w, &m))
    }
}

impl EmitterPlugin<CSharp> for RequestIdPlugin {
    fn after_type(&self, w: &mut dyn IndentWrite, ctx: &EmitContext) -> io::Result<()> {
        self.matched(ctx)
            .map_or(Ok(()), |m| csharp::emit(w, &m, ctx))
    }
}
