//! C#: a `RequestKind` enum and a `RequestKind` property on the effect record.
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
    writeln!(w, "public enum RequestKind")?;
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
    writeln!(w, "public {ns}.RequestKind? RequestKind => this switch")?;
    writeln!(w, "{{")?;
    w.indent();
    for variant in &m.variants {
        let name = variant.name;
        let kind = match variant.kind {
            Some(crate::RequestKind::Notify) => format!("{ns}.RequestKind.Notify"),
            Some(crate::RequestKind::Request) => format!("{ns}.RequestKind.Request"),
            Some(crate::RequestKind::Stream) => format!("{ns}.RequestKind.Stream"),
            None => "null".to_string(),
        };
        writeln!(w, "{ns}.{effect}.{name} => {kind},")?;
    }
    writeln!(w, "_ => null,")?;
    w.unindent();
    writeln!(w, "}};")?;

    Ok(())
}
