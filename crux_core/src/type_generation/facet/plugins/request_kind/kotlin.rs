//! Kotlin: a `RequestKind` enum class and a `requestKind` extension property
//! on the effect.

use std::io;

use facet_generate::generation::{
    indent::IndentWrite, kotlin::variant_class_name, plugin::EmitContext,
};

use super::Matched;

pub(super) fn emit(w: &mut dyn IndentWrite, m: &Matched<'_>, _ctx: &EmitContext) -> io::Result<()> {
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
    writeln!(w, "enum class RequestKind {{")?;
    w.indent();
    writeln!(w, "/// Never — the core is not waiting for an answer.")?;
    writeln!(w, "NOTIFY,")?;
    writeln!(w, "/// Exactly once.")?;
    writeln!(w, "REQUEST,")?;
    writeln!(w, "/// Any number of times, until the shell stops sending.")?;
    writeln!(w, "STREAM,")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_accessor(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    let effect = m.name;

    writeln!(w)?;
    writeln!(
        w,
        "/// How many times the shell resolves a request carrying this effect,"
    )?;
    writeln!(
        w,
        "/// or `null` when the operation leaves that to the call site."
    )?;
    writeln!(w, "val {effect}.requestKind: RequestKind?")?;
    w.indent();
    writeln!(w, "get() = when (this) {{")?;
    w.indent();
    for variant in &m.variants {
        let class = variant_class_name(variant.name);
        let kind = match variant.kind {
            Some(crate::RequestKind::Notify) => "RequestKind.NOTIFY",
            Some(crate::RequestKind::Request) => "RequestKind.REQUEST",
            Some(crate::RequestKind::Stream) => "RequestKind.STREAM",
            None => "null",
        };
        writeln!(w, "is {effect}.{class} -> {kind}")?;
    }
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();

    Ok(())
}
