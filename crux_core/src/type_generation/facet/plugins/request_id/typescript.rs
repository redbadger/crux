//! TypeScript: an `EffectKind` union — the same discriminant the effect union
//! already uses — and a `decodeRequestId` function.

use std::io;

use facet_generate::generation::indent::IndentWrite;

use super::{EFFECT_SHIFT, Matched, SEQUENCE_MASK, STREAM_BIT};

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    emit_effect_kind(w, m)?;
    emit_request_id(w)
}

fn emit_effect_kind(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    let union = m
        .variants
        .iter()
        .map(|variant| format!(r#""{}""#, variant.name))
        .collect::<Vec<_>>()
        .join(" | ");
    let list = m
        .variants
        .iter()
        .map(|variant| format!(r#""{}""#, variant.name))
        .collect::<Vec<_>>()
        .join(", ");

    writeln!(w)?;
    writeln!(
        w,
        "/// Which effect a request id names — the same discriminant `{}` uses.",
        m.name
    )?;
    writeln!(w, "export type EffectKind = {union};")?;
    writeln!(w)?;
    writeln!(
        w,
        "/// The effects in declaration order, indexed by the position a request id"
    )?;
    writeln!(w, "/// carries.")?;
    writeln!(w, "const EFFECT_KINDS: EffectKind[] = [{list}];")?;

    Ok(())
}

fn emit_request_id(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// The parts of the id the core issues with every request."
    )?;
    writeln!(w, "export interface RequestId {{")?;
    w.indent();
    writeln!(
        w,
        "/// The id as it came from the core, and as it goes back."
    )?;
    writeln!(w, "rawValue: number;")?;
    writeln!(
        w,
        "/// Every notification shares one reserved id, because the core is not"
    )?;
    writeln!(w, "/// waiting on any of them.")?;
    writeln!(w, "isNotification: boolean;")?;
    writeln!(
        w,
        "/// Which effect this request carries — `undefined` for a notification,"
    )?;
    writeln!(
        w,
        "/// whose id names none, and for an id naming an effect this generated"
    )?;
    writeln!(w, "/// code does not know.")?;
    writeln!(w, "effectKind: EffectKind | undefined;")?;
    writeln!(
        w,
        "/// How many times the core expects this request to be resolved."
    )?;
    writeln!(w, "requestKind: RequestKind;")?;
    writeln!(
        w,
        "/// The ascending number the request was issued with. Zero for a"
    )?;
    writeln!(w, "/// notification, the one id that is not sequenced.")?;
    writeln!(w, "sequence: number;")?;
    w.unindent();
    writeln!(w, "}}")?;

    writeln!(w)?;
    writeln!(w, "/// Read a request id.")?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// Resolve with the id exactly as it arrived — this is for logging, tracing"
    )?;
    writeln!(
        w,
        "/// and assertions. The bit layout is Crux's, so decode ids with this rather"
    )?;
    writeln!(w, "/// than picking the number apart by hand.")?;
    writeln!(
        w,
        "export function decodeRequestId(rawValue: number): RequestId {{"
    )?;
    w.indent();
    writeln!(w, "if (rawValue === 0) {{")?;
    w.indent();
    writeln!(w, "return {{")?;
    w.indent();
    writeln!(w, "rawValue,")?;
    writeln!(w, "isNotification: true,")?;
    writeln!(w, "effectKind: undefined,")?;
    writeln!(w, r#"requestKind: "notify","#)?;
    writeln!(w, "sequence: 0,")?;
    w.unindent();
    writeln!(w, "}};")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "return {{")?;
    w.indent();
    writeln!(w, "rawValue,")?;
    writeln!(w, "isNotification: false,")?;
    writeln!(
        w,
        "effectKind: EFFECT_KINDS[(rawValue >>> {EFFECT_SHIFT}) & 0xff],"
    )?;
    writeln!(
        w,
        r#"requestKind: (rawValue & {STREAM_BIT:#x}) === 0 ? "request" : "stream","#
    )?;
    writeln!(w, "sequence: rawValue & {SEQUENCE_MASK:#x},")?;
    w.unindent();
    writeln!(w, "}};")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
