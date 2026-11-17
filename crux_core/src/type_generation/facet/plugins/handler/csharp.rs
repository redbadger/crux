//! C#: `IEffectSink`, the `IEffectHandler` interface and `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::csharp::write_serialize_value,
    csharp::{escape_identifier, render_type, write_doc},
    indent::IndentWrite,
};

use super::{
    super::{Member, Variant, lower_camel},
    Matched,
};

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
    let members = m.members();
    if !members.is_empty() {
        return emit_grouped_handler(w, m, &members, config);
    }

    writeln!(w)?;
    emit_handler_summary(w, m, None)?;
    writeln!(w, "public interface IEffectHandler")?;
    writeln!(w, "{{")?;
    w.indent();
    for variant in &m.variants {
        emit_method(w, variant, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The doc comment of `IEffectHandler`, with an extra line when the methods
/// are in the group interfaces.
fn emit_handler_summary(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    extra: Option<&str>,
) -> io::Result<()> {
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
    if let Some(extra) = extra {
        writeln!(w, "///")?;
        writeln!(w, "/// {extra}")?;
    }
    writeln!(w, "/// </summary>")
}

/// One method of a handler interface. A method other than `Render` carries
/// the doc comment of its operation, and `Render` does nothing by default.
fn emit_method(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    // `Core` handles the render operation, and its method has a doc of its
    // own, so only the other operations carry the operation's doc.
    if !variant.render {
        write_doc(w, variant.doc)?;
    }
    if variant.render && !variant.is_legacy() && !variant.is_stream() && !variant.is_request() {
        let method = escape_identifier(variant.name);
        let operation = render_type(variant.operation, config);
        writeln!(w, "/// <summary>")?;
        writeln!(
            w,
            "/// <c>Core</c> handles <c>{}</c> itself, so this does nothing;",
            variant.name
        )?;
        writeln!(
            w,
            "/// implement it only if you drive <c>EffectDispatcher</c> yourself."
        )?;
        writeln!(w, "/// </summary>")?;
        writeln!(w, "void {method}({operation} operation) {{ }}")?;
    } else {
        writeln!(w, "{};", signature(variant, config))?;
    }

    Ok(())
}

/// The signature of the handler method for `variant`, without the `;` of an
/// interface member.
fn signature(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    let method = escape_identifier(variant.name);
    let operation = render_type(variant.operation, config);
    if variant.is_legacy() {
        format!("void {method}({operation} operation, uint requestId, Action<byte[]> resolve)")
    } else if variant.is_stream() {
        let output = render_output(variant, config);
        format!("void {method}({operation} operation, IEffectSink<{output}> sink)")
    } else if variant.is_request() {
        let output = render_output(variant, config);
        format!("Task<{output}> {method}({operation} operation)")
    } else {
        format!("void {method}({operation} operation)")
    }
}

/// The handler API when the app puts its effects into groups: one interface
/// for each group, `IEffectHandler` that inherits them all, and
/// `GroupedEffectHandler`, a class that forwards each method to the handler
/// of its group.
///
/// The handlers are in private fields with an underscore prefix, because a
/// property cannot have the name of a method.
fn emit_grouped_handler(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    members: &[Member<'_>],
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    for member in members {
        writeln!(w)?;
        writeln!(w, "/// <summary>")?;
        if member.is_ungrouped() {
            writeln!(
                w,
                "/// Handles the effects of <c>{}</c> that are in no group.",
                m.name
            )?;
        } else {
            writeln!(
                w,
                "/// Handles the effects of <c>{}</c> in the <c>{}</c> group.",
                m.name, member.name
            )?;
        }
        writeln!(w, "/// </summary>")?;
        writeln!(w, "public interface I{}", member.handler())?;
        writeln!(w, "{{")?;
        w.indent();
        for variant in &member.variants {
            emit_method(w, variant, config)?;
        }
        w.unindent();
        writeln!(w, "}}")?;
    }

    writeln!(w)?;
    emit_handler_summary(
        w,
        m,
        Some("The methods are in the group interfaces that this interface inherits."),
    )?;
    let handlers: Vec<String> = members
        .iter()
        .map(|member| format!("I{}", member.handler()))
        .collect();
    writeln!(
        w,
        "public interface IEffectHandler : {}",
        handlers.join(", ")
    )?;
    writeln!(w, "{{")?;
    w.indent();
    if let Some(render) = m.render_variant() {
        emit_method(w, render, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    emit_grouped_class(w, members, config)
}

/// `GroupedEffectHandler`, which forwards each method to the handler of its
/// group.
fn emit_grouped_class(
    w: &mut dyn IndentWrite,
    members: &[Member<'_>],
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// An <c>IEffectHandler</c> made from one handler for each group. Each"
    )?;
    writeln!(w, "/// method forwards to the handler of its group.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(
        w,
        "public sealed class GroupedEffectHandler : IEffectHandler"
    )?;
    writeln!(w, "{{")?;
    w.indent();
    for member in members {
        writeln!(
            w,
            "private readonly I{} _{};",
            member.handler(),
            member.parameter()
        )?;
    }
    writeln!(w)?;
    let parameters: Vec<String> = members
        .iter()
        .map(|member| {
            format!(
                "I{} {}",
                member.handler(),
                escape_identifier(&member.parameter())
            )
        })
        .collect();
    writeln!(w, "public GroupedEffectHandler({})", parameters.join(", "))?;
    writeln!(w, "{{")?;
    w.indent();
    for member in members {
        let parameter = member.parameter();
        writeln!(w, "_{parameter} = {};", escape_identifier(&parameter))?;
    }
    w.unindent();
    writeln!(w, "}}")?;
    for member in members {
        let target = format!("_{}", member.parameter());
        for variant in &member.variants {
            writeln!(w)?;
            writeln!(
                w,
                "public {} => {};",
                signature(variant, config),
                forward(&target, variant)
            )?;
        }
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The call that forwards `variant` to the same method on `target`.
fn forward(target: &str, variant: &Variant<'_>) -> String {
    let method = escape_identifier(variant.name);
    if variant.is_legacy() {
        format!("{target}.{method}(operation, requestId, resolve)")
    } else if variant.is_stream() {
        format!("{target}.{method}(operation, sink)")
    } else {
        format!("{target}.{method}(operation)")
    }
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
    if let Some(output) = &variant.output {
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
    variant.output.as_ref().map_or_else(
        || "object".to_string(),
        |format| render_type(format, config),
    )
}
