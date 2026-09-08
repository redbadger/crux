//! TypeScript: a `RequestKind` string union and a free function that reads it
//! off an effect.
//!
//! The union that facet-generate emits for an enum already discriminates on
//! `kind`, so the accessor is a function rather than a property.

use std::io;

use facet_generate::generation::{indent::IndentWrite, plugin::EmitContext};

use super::Matched;
use crate::type_generation::facet::plugins::lower_camel;

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
        "/// How many times the shell is expected to resolve a request:"
    )?;
    writeln!(
        w,
        "/// `notify` never, `request` exactly once, `stream` any number of times."
    )?;
    writeln!(
        w,
        r#"export type RequestKind = "notify" | "request" | "stream";"#
    )?;

    Ok(())
}

fn emit_accessor(w: &mut dyn IndentWrite, m: &Matched<'_>) -> io::Result<()> {
    let effect = m.name;
    let function = format!("{}RequestKind", lower_camel(effect));

    writeln!(w)?;
    writeln!(
        w,
        "/// How many times the shell resolves a request carrying this effect,"
    )?;
    writeln!(
        w,
        "/// or `undefined` when the operation leaves that to the call site."
    )?;
    writeln!(
        w,
        "export function {function}(effect: {effect}): RequestKind | undefined {{"
    )?;
    w.indent();
    writeln!(w, "switch (effect.kind) {{")?;
    w.indent();
    for variant in &m.variants {
        let name = variant.name;
        let kind = match variant.kind {
            Some(crate::RequestKind::Notify) => r#""notify""#,
            Some(crate::RequestKind::Request) => r#""request""#,
            Some(crate::RequestKind::Stream) => r#""stream""#,
            None => "undefined",
        };
        writeln!(w, r#"case "{name}": return {kind};"#)?;
    }
    writeln!(w, "default: return undefined;")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
