//! The `EmitterPlugin`s Crux adds on top of facet-generate's
//! [`BincodePlugin`](facet_generate::generation::bincode::BincodePlugin).
//!
//! * [`RequestKindPlugin`] emits the `RequestKind` type and the accessor that
//!   answers "how many times does the shell resolve this effect?".
//! * [`EffectHandlerPlugin`] emits `EffectSink`, `EffectHandler` and
//!   `EffectDispatcher` — a shell writes the handler, the dispatcher does the
//!   resolving.
//! * [`RequestIdPlugin`] emits `EffectKind` and `RequestId`, which read the
//!   structured request id the core issues.
//! * [`CorePlugin`] emits `CoreBridge` and `Core` — the loop that carries
//!   events into the core, requests out of it, and the view back to the shell.
//!
//! They act only when the container being emitted is a registered effect enum
//! (see [`Matched`]), and they write everything through the `after_type` hook,
//! so the generated declarations land beside the effect they belong to. The
//! one exception is C#, where the request-kind accessor is a property *inside*
//! the `Effect` record and so goes through `type_body` — the emitter does not
//! declare the record `partial`, so it cannot be re-opened from outside.

mod core;
mod handler;
mod request_id;
mod request_kind;
#[cfg(test)]
mod tests;

use facet_generate::{
    generation::{CodeGeneratorConfig, PackageLocation, plugin::EmitContext},
    reflection::format::{ContainerFormat, Format, VariantFormat},
};
use heck::ToLowerCamelCase;

pub(super) use core::CorePlugin;
pub(super) use handler::EffectHandlerPlugin;
pub(super) use request_id::RequestIdPlugin;
pub(super) use request_kind::RequestKindPlugin;

use super::{EffectMeta, EffectVariantMeta};
use crate::RequestKind;

/// The generated Swift package manifest does not declare platforms, so it
/// defaults to a deployment target older than Swift concurrency. Requests are
/// dispatched in a `Task`, so the generated API has to say when it is
/// available.
pub const SWIFT_AVAILABILITY: &str = "@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)";

/// One variant of an effect enum, as the plugins see it: the registry's view
/// of the variant (its emitted name and payload type) paired with what the
/// operation declared.
#[derive(Debug, Clone, Copy)]
pub struct Variant<'a> {
    /// The variant name the emitters use, after any rename.
    pub name: &'a str,
    /// The kind the operation declares, or `None` for a legacy operation whose
    /// kind is decided by the call site.
    pub kind: Option<RequestKind>,
    /// The type the request resolves with. `None` for a notification.
    pub output: Option<&'a Format>,
    /// The operation type the variant carries.
    pub operation: &'a Format,
    /// Whether the operation is `crux_core::render::RenderOperation`, which the
    /// generated `Core` handles itself.
    pub render: bool,
}

/// The effect enum a plugin hook was called for.
#[derive(Debug)]
pub struct Matched<'a> {
    /// The effect enum's emitted name.
    pub name: &'a str,
    /// Whether this is the first registered effect. The `RequestKind` type and
    /// the handler API use fixed names, so they are emitted only once per
    /// generated package.
    pub primary: bool,
    pub variants: Vec<Variant<'a>>,
}

/// Pair the container being emitted with the effect metadata recorded for it,
/// or `None` if this container is not a registered effect enum.
///
/// Every variant has to line up — the generated `switch` is exhaustive, so a
/// half-understood effect is worse than none at all.
pub fn matched<'a>(effects: &'a [EffectMeta], ctx: &EmitContext<'a>) -> Option<Matched<'a>> {
    let index = effects
        .iter()
        .position(|e| &e.effect == ctx.container.name)?;
    let ContainerFormat::Enum(registry_variants, _, _) = ctx.container.format else {
        return None;
    };

    let recorded = &effects[index].variants;
    if recorded.len() != registry_variants.len() {
        return None;
    }

    let mut variants = Vec::with_capacity(recorded.len());
    for meta in recorded {
        let named = registry_variants.get(&u32::try_from(meta.index).ok()?)?;
        let VariantFormat::NewType(operation) = &named.value else {
            return None;
        };
        variants.push(Variant {
            name: named.name.as_str(),
            kind: meta.kind,
            output: output_of(meta),
            operation: operation.as_ref(),
            render: meta.render,
        });
    }

    Some(Matched {
        name: ctx.container.name.name.as_str(),
        primary: index == 0,
        variants,
    })
}

/// The output format to generate a signature from.
///
/// A notification is never resolved, and an operation that declares no kind is
/// resolved by the shell with bytes it produces itself, so neither has an
/// output the generated API names.
const fn output_of(meta: &EffectVariantMeta) -> Option<&Format> {
    match (&meta.output, meta.kind) {
        (Some(format), Some(RequestKind::Request | RequestKind::Stream)) => Some(format),
        _ => None,
    }
}

/// The lower-camel-cased form the Kotlin, Swift and TypeScript emitters use
/// for a member derived from a type or variant name.
pub fn lower_camel(name: &str) -> String {
    name.to_lower_camel_case()
}

/// Whether the dispatcher will serialize an output, which is what needs a
/// serializer imported.
pub fn serializes_output(effects: &[EffectMeta]) -> bool {
    effects.first().is_some_and(|effect| {
        effect.variants.iter().any(|variant| {
            matches!(
                variant.kind,
                Some(RequestKind::Request | RequestKind::Stream)
            )
        })
    })
}

/// Where the TypeScript bincode runtime lives, resolved the same way the
/// bincode plugin resolves the serde runtime.
pub fn bincode_import_path(config: &CodeGeneratorConfig) -> String {
    config.external_packages.get("bincode").map_or_else(
        || "./bincode".to_string(),
        |package| match &package.location {
            PackageLocation::Path(_) => {
                let name = &package.for_namespace;
                package
                    .module_name
                    .as_ref()
                    .map_or_else(|| name.clone(), |module| format!("{name}/{module}"))
            }
            PackageLocation::Url(_) => package.for_namespace.clone(),
        },
    )
}

impl<'a> Matched<'a> {
    /// The variant carrying `RenderOperation`, if the effect has one.
    ///
    /// The generated `Core` is only emitted for an effect that has one, since
    /// its whole job is to own the view loop.
    pub fn render_variant(&self) -> Option<&Variant<'a>> {
        self.variants.iter().find(|variant| variant.render)
    }
}

impl Variant<'_> {
    /// Whether the shell resolves this request exactly once with a typed
    /// output.
    pub const fn is_request(&self) -> bool {
        matches!(self.kind, Some(RequestKind::Request))
    }

    /// Whether the shell resolves this request many times with a typed output.
    pub const fn is_stream(&self) -> bool {
        matches!(self.kind, Some(RequestKind::Stream))
    }

    /// Whether the operation leaves the kind to the call site, so the shell
    /// gets the raw request id and resolves it by hand.
    pub const fn is_legacy(&self) -> bool {
        self.kind.is_none()
    }
}
