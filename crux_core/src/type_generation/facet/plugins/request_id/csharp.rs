//! C#: an `EffectKind` enum backed by the variant index, and a `RequestId`
//! record that decodes an id into its parts.

use std::io;

use facet_generate::generation::{
    csharp::escape_identifier, indent::IndentWrite, plugin::EmitContext,
};

use super::{EFFECT_SHIFT, Matched, SEQUENCE_MASK, STREAM_BIT};

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>, ctx: &EmitContext) -> io::Result<()> {
    emit_effect_kind(w, m)?;
    emit_request_id(w, m, ctx)
}

fn emit_effect_kind(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(w, "/// Which effect a request id names. The value is the")?;
    writeln!(
        w,
        "/// effect's position in <c>{}</c>, which is what the request",
        m.name
    )?;
    writeln!(w, "/// id carries.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public enum EffectKind : byte")?;
    writeln!(w, "{{")?;
    w.indent();
    for (index, variant) in m.variants.iter().enumerate() {
        writeln!(w, "{} = {index},", escape_identifier(variant.name))?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_request_id(w: &mut dyn IndentWrite, m: &Matched<'_>, ctx: &EmitContext) -> io::Result<()> {
    let ns = &ctx.config.module_name;

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// The parts of the id the core issues with every request."
    )?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// Resolve with <c>RawValue</c>, exactly as it arrived — this is for"
    )?;
    writeln!(
        w,
        "/// logging, tracing and assertions. The bit layout is Crux's, so read"
    )?;
    writeln!(
        w,
        "/// ids with this rather than picking the integer apart by hand."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public sealed record RequestId(uint RawValue)")?;
    writeln!(w, "{{")?;
    w.indent();

    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Every notification shares one reserved id, because the core is"
    )?;
    writeln!(w, "/// not waiting on any of them.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public bool IsNotification => RawValue == 0;")?;
    writeln!(w)?;

    emit_effect_kind_property(w, m, ns)?;
    writeln!(w)?;
    emit_request_kind_property(w, ns)?;
    writeln!(w)?;

    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// The ascending number the request was issued with. Zero for a"
    )?;
    writeln!(w, "/// notification, the one id that is not sequenced.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public uint Sequence => RawValue & {SEQUENCE_MASK:#x}u;")?;

    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// Which effect the id names, decoded from its variant index.
fn emit_effect_kind_property(w: &mut dyn IndentWrite, m: &Matched<'_>, ns: &str) -> io::Result<()> {
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Which effect this request carries — <c>null</c> for a notification,"
    )?;
    writeln!(
        w,
        "/// whose id names none, and for an id naming an effect this generated"
    )?;
    writeln!(w, "/// code does not know.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public {ns}.EffectKind? EffectKind")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "get")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "if (IsNotification)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "return null;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "return (RawValue >> {EFFECT_SHIFT}) switch")?;
    writeln!(w, "{{")?;
    w.indent();
    for (index, variant) in m.variants.iter().enumerate() {
        writeln!(
            w,
            "{index}u => {ns}.EffectKind.{},",
            escape_identifier(variant.name)
        )?;
    }
    writeln!(w, "_ => null,")?;
    w.unindent();
    writeln!(w, "}};")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")
}

/// How many times the core expects the request to be resolved.
fn emit_request_kind_property(w: &mut dyn IndentWrite, ns: &str) -> io::Result<()> {
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// How many times the core expects this request to be resolved."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public {ns}.RequestKind RequestKind")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "get")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "if (IsNotification)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "return {ns}.RequestKind.Notify;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(
        w,
        "return (RawValue & {STREAM_BIT:#x}u) == 0 ? {ns}.RequestKind.Request : {ns}.RequestKind.Stream;"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")
}
