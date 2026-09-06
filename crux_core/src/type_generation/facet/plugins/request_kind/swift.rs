//! Swift: a `RequestKind` enum and a computed `requestKind` property in an
//! extension on the effect.

use std::io;

use facet_generate::generation::{indent::IndentWrite, swift::case_name};

use super::Matched;

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    if m.primary {
        emit_kind(w)?;
    }
    emit_accessor(w, m)
}

fn emit_kind(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// How many times the shell is expected to resolve a request."
    )?;
    writeln!(w, "public enum RequestKind: Hashable, Sendable {{")?;
    w.indent();
    writeln!(w, "/// Never — the core is not waiting for an answer.")?;
    writeln!(w, "case notify")?;
    writeln!(w, "/// Exactly once.")?;
    writeln!(w, "case request")?;
    writeln!(w, "/// Any number of times, until the shell stops sending.")?;
    writeln!(w, "case stream")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_accessor(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "extension {} {{", m.name)?;
    w.indent();
    writeln!(
        w,
        "/// How many times the shell resolves a request carrying this effect,"
    )?;
    writeln!(
        w,
        "/// or `nil` when the operation leaves that to the call site."
    )?;
    writeln!(w, "public var requestKind: RequestKind? {{")?;
    w.indent();
    writeln!(w, "switch self {{")?;
    for variant in &m.variants {
        let case = case_name(variant.name);
        let kind = match variant.kind {
            Some(crate::RequestKind::Notify) => ".notify",
            Some(crate::RequestKind::Request) => ".request",
            Some(crate::RequestKind::Stream) => ".stream",
            None => "nil",
        };
        writeln!(w, "case .{case}: return {kind}")?;
    }
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
