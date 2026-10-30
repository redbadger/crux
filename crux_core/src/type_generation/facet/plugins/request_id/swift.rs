//! Swift: an `EffectKind` enum raw-valued by variant index, and a `RequestId`
//! struct that decodes an id into its parts.

use std::io;

use facet_generate::generation::{
    indent::IndentWrite,
    swift::{case_name, write_doc},
};

use super::{EFFECT_SHIFT, Matched, SEQUENCE_MASK, STREAM_BIT};

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    emit_effect_kind(w, m)?;
    emit_request_id(w)
}

fn emit_effect_kind(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    writeln!(w)?;
    write_doc(
        w,
        &[
            "Which effect a request id names.",
            "",
            &format!(
                "The raw value is the effect's position in `{}`, which is what the",
                m.name
            ),
            "request id carries.",
        ],
    )?;
    writeln!(w, "public enum EffectKind: UInt8, Hashable, Sendable {{")?;
    w.indent();
    for (index, variant) in m.variants.iter().enumerate() {
        writeln!(w, "case {} = {index}", case_name(variant.name))?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_request_id(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    write_doc(
        w,
        &[
            "The parts of the id the core issues with every request.",
            "",
            "Resolve with `rawValue`, exactly as it arrived — this is for logging,",
            "tracing and assertions. The bit layout is Crux's, so read ids with this",
            "rather than picking the integer apart by hand.",
        ],
    )?;
    writeln!(w, "public struct RequestId: Hashable, Sendable {{")?;
    w.indent();
    write_doc(
        w,
        &["The id as it came from the core, and as it goes back."],
    )?;
    writeln!(w, "public let rawValue: UInt32")?;
    writeln!(w)?;
    writeln!(w, "public init(_ rawValue: UInt32) {{")?;
    w.indent();
    writeln!(w, "self.rawValue = rawValue")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    write_doc(
        w,
        &[
            "Every notification shares one reserved id, because the core is not",
            "waiting on any of them.",
        ],
    )?;
    writeln!(w, "public var isNotification: Bool {{")?;
    w.indent();
    writeln!(w, "self.rawValue == 0")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    write_doc(
        w,
        &[
            "Which effect this request carries — `nil` for a notification, whose",
            "id names none, and for an id naming an effect this generated code",
            "does not know.",
        ],
    )?;
    writeln!(w, "public var effectKind: EffectKind? {{")?;
    w.indent();
    writeln!(w, "if self.isNotification {{ return nil }}")?;
    writeln!(
        w,
        "return EffectKind(rawValue: UInt8(truncatingIfNeeded: self.rawValue >> {EFFECT_SHIFT}))"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    write_doc(
        w,
        &["How many times the core expects this request to be resolved."],
    )?;
    writeln!(w, "public var operationKind: OperationKind {{")?;
    w.indent();
    writeln!(w, "if self.isNotification {{ return .notify }}")?;
    writeln!(
        w,
        "return (self.rawValue & {STREAM_BIT:#x}) == 0 ? .request : .stream"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    write_doc(
        w,
        &[
            "The ascending number the request was issued with. Zero for a",
            "notification, the one id that is not sequenced.",
        ],
    )?;
    writeln!(w, "public var sequence: UInt32 {{")?;
    w.indent();
    writeln!(w, "self.rawValue & {SEQUENCE_MASK:#x}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
