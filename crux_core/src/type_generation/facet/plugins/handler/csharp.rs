//! C#: `IEffectSink`, the `IEffectHandler` interface and `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::csharp::write_serialize_value,
    csharp::{escape_identifier, render_type},
    indent::IndentWrite,
};

use super::{super::Variant, super::lower_camel, Matched};

pub(super) fn emit(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    emit_sink(w)?;
    emit_handler(w, m, config)?;
    emit_dispatcher(w, m, config)
}

fn emit_sink(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Receives the items of a streaming effect. Every item sent"
    )?;
    writeln!(
        w,
        "/// is one resolution of the request that started the stream."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public interface IEffectSink<in T>")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "void Send(T item);")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_handler(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(w, "/// Handles every effect <c>{}</c> can carry.", m.name)?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// <c>EffectDispatcher</c> resolves each request for you, so there is"
    )?;
    writeln!(
        w,
        "/// no <c>resolve</c> to call at the wrong time or with the wrong bytes."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public interface IEffectHandler")?;
    writeln!(w, "{{")?;
    w.indent();
    for variant in &m.variants {
        let method = escape_identifier(variant.name);
        let operation = render_type(variant.operation, config);
        if variant.is_legacy() {
            writeln!(
                w,
                "void {method}({operation} operation, uint requestId, Action<byte[]> resolve);"
            )?;
        } else if variant.is_stream() {
            let output = render_output(variant, config);
            writeln!(
                w,
                "void {method}({operation} operation, IEffectSink<{output}> sink);"
            )?;
        } else if variant.is_request() {
            let output = render_output(variant, config);
            writeln!(w, "Task<{output}> {method}({operation} operation);")?;
        } else {
            writeln!(w, "void {method}({operation} operation);")?;
        }
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_dispatcher(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let ns = &config.module_name;
    let effect = m.name;

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Routes each request to an <c>IEffectHandler</c> and resolves it for"
    )?;
    writeln!(
        w,
        "/// you: never for a notification, once for a request, once per item"
    )?;
    writeln!(w, "/// for a stream.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public sealed class EffectDispatcher")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "private readonly IEffectHandler _handler;")?;
    writeln!(w, "private readonly Action<uint, byte[]> _resolve;")?;
    writeln!(w)?;
    writeln!(
        w,
        "public EffectDispatcher(IEffectHandler handler, Action<uint, byte[]> resolve)"
    )?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_handler = handler;")?;
    writeln!(w, "_resolve = resolve;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "public void Dispatch({ns}.Request request)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "var id = request.Id;")?;
    writeln!(w, "switch (request.Effect)")?;
    writeln!(w, "{{")?;
    w.indent();
    for variant in &m.variants {
        let binding = escape_identifier(&lower_camel(variant.name)).into_owned();
        writeln!(w, "case {ns}.{effect}.{} {binding}:", variant.name)?;
        w.indent();
        emit_arm(w, variant, &binding, config)?;
        writeln!(w, "break;")?;
        w.unindent();
    }
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    // Request variants are dispatched through a private async method so that
    // `Dispatch` itself stays synchronous.
    for variant in &m.variants {
        if variant.is_request() {
            emit_async_helper(w, variant, config)?;
        }
    }

    emit_delegate_sink(w)?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_arm(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    binding: &str,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let method = escape_identifier(variant.name);

    if variant.is_legacy() {
        writeln!(
            w,
            "_handler.{method}({binding}.Value, id, bytes => _resolve(id, bytes));"
        )?;
    } else if variant.is_stream() {
        let output = render_output(variant, config);
        writeln!(
            w,
            "_handler.{method}({binding}.Value, new DelegateEffectSink<{output}>(item =>"
        )?;
        writeln!(w, "{{")?;
        w.indent();
        emit_resolve(w, "item", variant, config)?;
        w.unindent();
        writeln!(w, "}}));")?;
    } else if variant.is_request() {
        writeln!(w, "_ = Dispatch{}({binding}.Value, id);", variant.name)?;
    } else {
        writeln!(w, "_handler.{method}({binding}.Value);")?;
    }

    Ok(())
}

fn emit_async_helper(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let method = escape_identifier(variant.name);
    let operation = render_type(variant.operation, config);

    writeln!(w)?;
    writeln!(
        w,
        "private async Task Dispatch{}({operation} operation, uint id)",
        variant.name
    )?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "var output = await _handler.{method}(operation);")?;
    emit_resolve(w, "output", variant, config)?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_resolve(
    w: &mut dyn IndentWrite,
    value: &str,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w, "var serializer = new BincodeSerializer();")?;
    if let Some(output) = variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    writeln!(w, "_resolve(id, serializer.GetBytes());")?;

    Ok(())
}

/// C# has no anonymous interface implementations, so a stream's sink is a
/// small adapter over a delegate.
fn emit_delegate_sink(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "private sealed class DelegateEffectSink<T> : IEffectSink<T>"
    )?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "private readonly Action<T> _send;")?;
    writeln!(w)?;
    writeln!(w, "public DelegateEffectSink(Action<T> send)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_send = send;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "public void Send(T item)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_send(item);")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn render_output(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant.output.map_or_else(
        || "object".to_string(),
        |format| render_type(format, config),
    )
}
