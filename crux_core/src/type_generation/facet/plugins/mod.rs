//! The `EmitterPlugin`s Crux adds on top of facet-generate's
//! [`BincodePlugin`](facet_generate::generation::bincode::BincodePlugin).
//!
//! * [`OperationKindPlugin`] emits the `OperationKind` type and the accessor that
//!   answers "how many times does the shell resolve this effect?".
//! * [`EffectHandlerPlugin`] emits `EffectSink`, `EffectHandler` and
//!   `EffectDispatcher` — a shell writes the handler, the dispatcher does the
//!   resolving.
//! * [`CorePlugin`] emits `CoreBridge` and `Core` — the loop that carries
//!   events into the core, requests out of it, and the view back to the shell.
//! * [`ShellHandlerPlugin`] emits the shell handlers a capability ships, for
//!   the app that asked for them — source the capability wrote, not source we
//!   generate.
//!
//! They act only when the container being emitted is a registered effect enum
//! (see [`Matched`]), and they write everything through the `after_type` hook,
//! so the generated declarations land beside the effect they belong to. The
//! one exception is C#, where the operation-kind accessor is a property *inside*
//! the `Effect` record and so goes through `type_body` — the emitter does not
//! declare the record `partial`, so it cannot be re-opened from outside.

mod core;
mod handler;
mod operation_kind;
mod shell_handler;
#[cfg(test)]
mod tests;

use facet_generate::{
    generation::{
        CodeGeneratorConfig, PackageLocation,
        csharp::{self, CSharp},
        kotlin::{self, Kotlin},
        plugin::EmitContext,
        swift::{self, Swift},
        typescript::{self, TypeScript},
    },
    reflection::format::{
        ContainerFormat, Format, FormatHolder as _, QualifiedTypeName, VariantFormat,
    },
};
use heck::ToLowerCamelCase;

pub(super) use core::CorePlugin;
pub(super) use handler::EffectHandlerPlugin;
pub(super) use operation_kind::OperationKindPlugin;
pub(super) use shell_handler::ShellHandlerPlugin;

use super::{EffectMeta, EffectVariantMeta};
use crate::OperationKind;

/// The generated Swift package manifest does not declare platforms, so it
/// defaults to a deployment target older than Swift concurrency. Requests are
/// dispatched in a `Task`, so the generated API has to say when it is
/// available.
pub const SWIFT_AVAILABILITY: &str = "@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)";

/// `Core` publishes its view model through the `Observation` framework, which
/// arrived later than Swift concurrency, so it alone carries a higher bar than
/// [`SWIFT_AVAILABILITY`].
pub const SWIFT_OBSERVABLE_AVAILABILITY: &str =
    "@available(macOS 14.0, iOS 17.0, tvOS 17.0, watchOS 10.0, *)";

/// How a language's emitter spells a reference to a type.
///
/// The formats an [`EmitContext`] carries are already in the emitter's
/// spelling, but a name Crux recorded itself — the app's event and view model,
/// an operation's output — is in registry spelling, and has to be respelled
/// exactly once before it is rendered or looked up in
/// [`CodeGeneratorConfig::is_enum`]. A second pass is not harmless: for C#,
/// Kotlin and TypeScript it qualifies an already-qualified name again.
pub trait Requalify {
    /// The language's `requalify`, e.g. [`csharp::requalify`].
    fn requalify(config: &CodeGeneratorConfig, name: &QualifiedTypeName) -> QualifiedTypeName;

    /// `format` with every type name in it respelled, by the language's
    /// `requalify_format`, e.g. [`csharp::requalify_format`].
    fn requalify_format(config: &CodeGeneratorConfig, format: &Format) -> Format;
}

impl Requalify for Swift {
    fn requalify(config: &CodeGeneratorConfig, name: &QualifiedTypeName) -> QualifiedTypeName {
        swift::requalify(config, name)
    }

    fn requalify_format(config: &CodeGeneratorConfig, format: &Format) -> Format {
        let mut format = format.clone();
        swift::requalify_format(config, &mut format);
        format
    }
}

impl Requalify for Kotlin {
    fn requalify(config: &CodeGeneratorConfig, name: &QualifiedTypeName) -> QualifiedTypeName {
        kotlin::requalify(config, name)
    }

    fn requalify_format(config: &CodeGeneratorConfig, format: &Format) -> Format {
        let mut format = format.clone();
        kotlin::requalify_format(config, &mut format);
        format
    }
}

impl Requalify for TypeScript {
    fn requalify(config: &CodeGeneratorConfig, name: &QualifiedTypeName) -> QualifiedTypeName {
        typescript::requalify(config, name)
    }

    fn requalify_format(config: &CodeGeneratorConfig, format: &Format) -> Format {
        let mut format = format.clone();
        typescript::requalify_format(config, &mut format);
        format
    }
}

impl Requalify for CSharp {
    fn requalify(config: &CodeGeneratorConfig, name: &QualifiedTypeName) -> QualifiedTypeName {
        csharp::requalify(config, name)
    }

    fn requalify_format(config: &CodeGeneratorConfig, format: &Format) -> Format {
        let mut format = format.clone();
        csharp::requalify_format(config, &mut format);
        format
    }
}

/// One variant of an effect enum, as the plugins see it: the registry's view
/// of the variant (its emitted name and payload type) paired with what the
/// operation declared.
#[derive(Debug, Clone)]
pub struct Variant<'a> {
    /// The variant name the emitters use, after any rename.
    pub name: &'a str,
    /// The kind the operation declares, or `None` for a legacy operation whose
    /// kind is decided by the call site.
    pub kind: Option<OperationKind>,
    /// The type the request resolves with, in the emitter's spelling. `None`
    /// for a notification.
    pub output: Option<Format>,
    /// The operation type the variant carries, from the registry the emitter
    /// has already respelled.
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
    /// Whether this is the first registered effect. The `OperationKind` type and
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
///
/// The container's name is still in registry spelling, as the effect metadata
/// is, so that is what they are compared in. Each output, recorded during
/// reflection, is respelled for `L` here, once.
pub fn matched<'a, L: Requalify>(
    effects: &'a [EffectMeta],
    ctx: &EmitContext<'a>,
) -> Option<Matched<'a>> {
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
            output: output_of(meta).map(|format| L::requalify_format(ctx.config, format)),
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
        (Some(format), Some(OperationKind::Request | OperationKind::Stream)) => Some(format),
        _ => None,
    }
}

/// Every type the generated handler API names as an output of `effect`, in
/// registry spelling: the output of each request and stream, and every type
/// inside one — an `Option<Presence>` names `Presence`.
///
/// Each is a registered type, since the outputs were reflected into the
/// registry the effect was recorded in.
pub fn output_types(effect: &EffectMeta) -> Vec<QualifiedTypeName> {
    let mut names = Vec::new();
    for format in effect.variants.iter().filter_map(output_of) {
        // A recorded output has no unresolved variables, which is all that
        // `visit` fails on.
        let _ = format.visit(&mut |format| {
            if let Format::TypeName(name) = format
                && !names.contains(name)
            {
                names.push(name.clone());
            }
            Ok(())
        });
    }
    names
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
                Some(OperationKind::Request | OperationKind::Stream)
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
        matches!(self.kind, Some(OperationKind::Request))
    }

    /// Whether the shell resolves this request many times with a typed output.
    pub const fn is_stream(&self) -> bool {
        matches!(self.kind, Some(OperationKind::Stream))
    }

    /// Whether the operation leaves the kind to the call site, so the shell
    /// gets the raw request id and resolves it by hand.
    pub const fn is_legacy(&self) -> bool {
        self.kind.is_none()
    }
}
