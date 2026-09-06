//! What type generation knows about an effect enum.
//!
//! `#[effect(facet_typegen)]` records one [`EffectMeta`] per effect enum, via
//! [`TypeRegistry::register_effect`](super::TypeRegistry::register_effect) and
//! the [`EffectBuilder`] it returns. The registry itself only knows the shape
//! of the types, so the two facts the plugins need — the
//! [`RequestKind`](crate::RequestKind) each variant declares and the type its
//! request resolves with — have to come from the operation types themselves.

use facet::Facet;
use facet_generate::reflection::format::{Format, QualifiedTypeName};

use super::{TypeGenError, TypeRegistry};
use crate::{RequestKind, capability::Operation};

/// Everything type generation knows about one effect enum, beyond its shape in
/// the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectMeta {
    /// The registry name of the effect enum, after any `#[facet(rename)]`.
    pub effect: QualifiedTypeName,
    /// One entry per variant, in declaration order.
    pub variants: Vec<EffectVariantMeta>,
}

/// One variant of an effect enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectVariantMeta {
    /// The variant's zero-based position, which is also its bincode
    /// discriminant. Used to pair this entry with the registry's own view of
    /// the variant, whose name may have been renamed.
    pub index: usize,
    /// The variant's Rust identifier.
    pub ident: String,
    /// The [`RequestKind`] the operation declares, or `None` for an operation
    /// that leaves the choice to the call site.
    pub kind: Option<RequestKind>,
    /// The format of `Operation::Output`, or `None` for a notification (which
    /// is never resolved, so its `()` output is not worth naming).
    pub output: Option<Format>,
}

/// Records the variants of one effect enum.
///
/// Returned by [`TypeRegistry::register_effect`]; call
/// [`variant`](Self::variant) once per variant, in declaration order, then
/// [`finish`](Self::finish).
pub struct EffectBuilder<'a> {
    registry: &'a mut TypeRegistry,
    meta: EffectMeta,
}

impl<'a> EffectBuilder<'a> {
    pub(super) const fn new(registry: &'a mut TypeRegistry, effect: QualifiedTypeName) -> Self {
        Self {
            registry,
            meta: EffectMeta {
                effect,
                variants: Vec::new(),
            },
        }
    }

    /// Record one variant, carrying operation `Op` under the Rust identifier
    /// `ident`.
    ///
    /// # Errors
    /// Returns a [`TypeGenError`] if `Op::Output` cannot be reflected.
    pub fn variant<'facet, Op>(mut self, ident: &str) -> Result<Self, TypeGenError>
    where
        Op: Operation,
        Op::Output: Facet<'facet>,
    {
        let kind = Op::KIND;

        // A notification is never resolved, so its output type is irrelevant —
        // and it is `()`, which no shell wants to see in a signature.
        let output = if kind == Some(RequestKind::Notify) {
            None
        } else {
            Some(
                self.registry
                    .builder
                    .format_of::<Op::Output>()
                    .map_err(|e| {
                        TypeGenError::Generation(format!(
                            "couldn't reflect the output of operation {}: {e}",
                            std::any::type_name::<Op>()
                        ))
                    })?,
            )
        };

        self.meta.variants.push(EffectVariantMeta {
            index: self.meta.variants.len(),
            ident: ident.to_string(),
            kind,
            output,
        });

        Ok(self)
    }

    /// Hand the recorded metadata back to the registry.
    pub fn finish(self) {
        let effect = self.meta.effect.clone();
        self.registry.effects.retain(|e| e.effect != effect);
        self.registry.effects.push(self.meta);
    }
}
