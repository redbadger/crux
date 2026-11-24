//! C#: an `OperationKind` enum and an `OperationKind` property on the effect record.
//!
//! The property goes inside the record: the emitter writes
//! `public abstract record Effect`, not `partial record`, so it cannot be
//! re-opened from outside the way a Swift or Kotlin extension can.

use std::io;

use facet_generate::generation::{indent::IndentWrite, plugin::EmitContext};

use super::Matched;

pub(super) fn emit_enum(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    if !m.primary {
        return Ok(());
    }

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// How many times the shell is expected to resolve a request."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public enum OperationKind")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "/// <summary>Never — the core is not waiting.</summary>")?;
    writeln!(w, "Notify,")?;
    writeln!(w, "/// <summary>Exactly once.</summary>")?;
    writeln!(w, "Request,")?;
    writeln!(
        w,
        "/// <summary>Any number of times, until the shell stops sending.</summary>"
    )?;
    writeln!(w, "Stream")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

pub(super) fn emit_accessor(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    ctx: &EmitContext,
) -> io::Result<()> {
    let ns = &ctx.config.module_name;
    let effect = m.name;

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// How many times the shell resolves a request carrying this effect,"
    )?;
    writeln!(
        w,
        "/// or <c>null</c> when the operation leaves that to the call site."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public {ns}.OperationKind? OperationKind => this switch")?;
    writeln!(w, "{{")?;
    w.indent();
    for variant in &m.variants {
        let name = variant.name;
        let kind = match variant.kind {
            Some(crate::OperationKind::Notify) => format!("{ns}.OperationKind.Notify"),
            Some(crate::OperationKind::Request) => format!("{ns}.OperationKind.Request"),
            Some(crate::OperationKind::Stream) => format!("{ns}.OperationKind.Stream"),
            None => "null".to_string(),
        };
        writeln!(w, "{ns}.{effect}.{name} => {kind},")?;
    }
    writeln!(w, "_ => null,")?;
    w.unindent();
    writeln!(w, "}};")?;

    Ok(())
}
