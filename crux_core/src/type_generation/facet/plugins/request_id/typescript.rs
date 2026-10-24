//! TypeScript: an `EffectKind` union — the same discriminant the effect union
//! already uses — and a `decodeRequestId` function.

use std::io;

use facet_generate::generation::{indent::IndentWrite, typescript::write_doc};

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
    write_doc(
        w,
        &[&format!(
            "Which effect a request id names — the same discriminant `{}` uses.",
            m.name
        )],
    )?;
    writeln!(w, "export type EffectKind = {union};")?;
    writeln!(w)?;
    write_doc(
        w,
        &[
            "The effects in declaration order, indexed by the position a request id",
            "carries.",
        ],
    )?;
    writeln!(w, "const EFFECT_KINDS: EffectKind[] = [{list}];")?;

    Ok(())
}

fn emit_request_id(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    write_doc(
        w,
        &["The parts of the id the core issues with every request."],
    )?;
    writeln!(w, "export interface RequestId {{")?;
    w.indent();
    write_doc(
        w,
        &["The id as it came from the core, and as it goes back."],
    )?;
    writeln!(w, "rawValue: number;")?;
    write_doc(
        w,
        &[
            "Every notification shares one reserved id, because the core is not",
            "waiting on any of them.",
        ],
    )?;
    writeln!(w, "isNotification: boolean;")?;
    write_doc(
        w,
        &[
            "Which effect this request carries — `undefined` for a notification,",
            "whose id names none, and for an id naming an effect this generated",
            "code does not know.",
        ],
    )?;
    writeln!(w, "effectKind: EffectKind | undefined;")?;
    write_doc(
        w,
        &["How many times the core expects this request to be resolved."],
    )?;
    writeln!(w, "operationKind: OperationKind;")?;
    write_doc(
        w,
        &[
            "The ascending number the request was issued with. Zero for a",
            "notification, the one id that is not sequenced.",
        ],
    )?;
    writeln!(w, "sequence: number;")?;
    w.unindent();
    writeln!(w, "}}")?;

    writeln!(w)?;
    write_doc(
        w,
        &[
            "Read a request id.",
            "",
            "Resolve with the id exactly as it arrived — this is for logging, tracing",
            "and assertions. The bit layout is Crux's, so decode ids with this rather",
            "than picking the number apart by hand.",
        ],
    )?;
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
    writeln!(w, r#"operationKind: "notify","#)?;
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
        r#"operationKind: (rawValue & {STREAM_BIT:#x}) === 0 ? "request" : "stream","#
    )?;
    writeln!(w, "sequence: rawValue & {SEQUENCE_MASK:#x},")?;
    w.unindent();
    writeln!(w, "}};")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
