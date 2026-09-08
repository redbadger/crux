//! Kotlin: an `EffectKind` enum class carrying the variant index, and a
//! `RequestId` value holding the decoded parts of an id.

use std::io;

use facet_generate::generation::{indent::IndentWrite, kotlin::variant_class_name};
use heck::ToShoutySnakeCase;

use super::{EFFECT_SHIFT, Matched, SEQUENCE_MASK, STREAM_BIT};

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    emit_effect_kind(w, m)?;
    emit_request_id(w)
}

fn emit_effect_kind(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// Which effect a request id names.")?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// `index` is the effect's position in `{}`, which is what the request",
        m.name
    )?;
    writeln!(w, "/// id carries.")?;
    writeln!(w, "enum class EffectKind(val index: UByte) {{")?;
    w.indent();
    let last = m.variants.len().saturating_sub(1);
    for (index, variant) in m.variants.iter().enumerate() {
        let name = variant_class_name(variant.name).to_shouty_snake_case();
        let end = if index == last { ";" } else { "," };
        writeln!(w, "{name}({index}u){end}")?;
    }
    writeln!(w)?;
    writeln!(w, "companion object {{")?;
    w.indent();
    writeln!(
        w,
        "/// The effect at `index`, or `null` if this generated code knows none."
    )?;
    writeln!(
        w,
        "fun fromIndex(index: UByte): EffectKind? = values().firstOrNull {{ it.index == index }}"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_request_id(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// The parts of the id the core issues with every request."
    )?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// Resolve with `rawValue`, exactly as it arrived — this is for logging,"
    )?;
    writeln!(
        w,
        "/// tracing and assertions. The bit layout is Crux's, so read ids with this"
    )?;
    writeln!(w, "/// rather than picking the integer apart by hand.")?;
    writeln!(w, "data class RequestId(val rawValue: UInt) {{")?;
    w.indent();
    writeln!(
        w,
        "/// Every notification shares one reserved id, because the core is not"
    )?;
    writeln!(w, "/// waiting on any of them.")?;
    writeln!(w, "val isNotification: Boolean")?;
    w.indent();
    writeln!(w, "get() = rawValue == 0u")?;
    w.unindent();
    writeln!(w)?;
    writeln!(
        w,
        "/// Which effect this request carries — `null` for a notification, whose"
    )?;
    writeln!(
        w,
        "/// id names none, and for an id naming an effect this generated code"
    )?;
    writeln!(w, "/// does not know.")?;
    writeln!(w, "val effectKind: EffectKind?")?;
    w.indent();
    writeln!(w, "get() = if (isNotification) null")?;
    writeln!(
        w,
        "else EffectKind.fromIndex((rawValue shr {EFFECT_SHIFT}).toUByte())"
    )?;
    w.unindent();
    writeln!(w)?;
    writeln!(
        w,
        "/// How many times the core expects this request to be resolved."
    )?;
    writeln!(w, "val requestKind: RequestKind")?;
    w.indent();
    writeln!(w, "get() = when {{")?;
    w.indent();
    writeln!(w, "isNotification -> RequestKind.NOTIFY")?;
    writeln!(
        w,
        "(rawValue and {STREAM_BIT:#x}u) == 0u -> RequestKind.REQUEST"
    )?;
    writeln!(w, "else -> RequestKind.STREAM")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w)?;
    writeln!(
        w,
        "/// The ascending number the request was issued with. Zero for a"
    )?;
    writeln!(w, "/// notification, the one id that is not sequenced.")?;
    writeln!(w, "val sequence: UInt")?;
    w.indent();
    writeln!(w, "get() = rawValue and {SEQUENCE_MASK:#x}u")?;
    w.unindent();
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
