//! TypeScript: `EffectSink`, the `EffectHandler` interface and
//! `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::typescript::write_serialize_value,
    indent::IndentWrite,
    typescript::{param_name, render_type, write_doc},
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
    write_doc(
        w,
        &[
            "Receives the items of a streaming effect.",
            "",
            "Every item sent is one resolution of the request that started the stream.",
        ],
    )?;
    writeln!(w, "export interface EffectSink<T> {{")?;
    w.indent();
    writeln!(w, "send(item: T): void;")?;
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
    write_doc(
        w,
        &[
            &format!("Handles every effect `{}` can carry.", m.name),
            "",
            "`EffectDispatcher` resolves each request for you, so there is no",
            "`resolve` to call at the wrong time or with the wrong bytes.",
        ],
    )?;
    writeln!(w, "export interface EffectHandler {{")?;
    w.indent();
    for variant in &m.variants {
        emit_method(w, variant, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// One method of a handler interface. A method other than `render` carries
/// the doc comment of its operation, and `render` is optional.
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
        write_doc(
            w,
            &[
                &format!(
                    "`Core` handles `{}` itself, so this is optional; implement it",
                    variant.name
                ),
                "only if you drive `EffectDispatcher` yourself.",
            ],
        )?;
        let method = lower_camel(variant.name);
        let operation = render_type(variant.operation, config);
        writeln!(w, "{method}?(operation: {operation}): void;")?;
    } else {
        writeln!(w, "{};", signature(variant, config))?;
    }

    Ok(())
}

/// The signature of the handler method for `variant`, without the `;` of
/// an interface member.
fn signature(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    let method = lower_camel(variant.name);
    let operation = render_type(variant.operation, config);
    if variant.is_legacy() {
        format!(
            "{method}(operation: {operation}, requestId: uint32, resolve: (bytes: Uint8Array) => void): void"
        )
    } else if variant.is_stream() {
        let output = render_output(variant, config);
        format!("{method}(operation: {operation}, sink: EffectSink<{output}>): void")
    } else if variant.is_request() {
        let output = render_output(variant, config);
        format!("{method}(operation: {operation}): Promise<{output}>")
    } else {
        format!("{method}(operation: {operation}): void")
    }
}

/// The handler API when the app puts its effects into groups: one interface
/// for each group, `EffectHandler` that extends them all, and
/// `GroupedEffectHandler`, a class that forwards each method to the handler
/// of its group.
///
/// The class forwards each method explicitly. An object spread of the group
/// handlers would not copy the methods of a class instance, because they are
/// on its prototype.
fn emit_grouped_handler(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    members: &[Member<'_>],
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    for member in members {
        writeln!(w)?;
        let doc = if member.is_ungrouped() {
            format!("Handles the effects of `{}` that are in no group.", m.name)
        } else {
            format!(
                "Handles the effects of `{}` in the `{}` group.",
                m.name, member.name
            )
        };
        write_doc(w, &[doc])?;
        writeln!(w, "export interface {} {{", member.handler())?;
        w.indent();
        for variant in &member.variants {
            emit_method(w, variant, config)?;
        }
        w.unindent();
        writeln!(w, "}}")?;
    }

    writeln!(w)?;
    write_doc(
        w,
        &[
            &format!("Handles every effect `{}` can carry.", m.name),
            "",
            "`EffectDispatcher` resolves each request for you, so there is no",
            "`resolve` to call at the wrong time or with the wrong bytes.",
            "",
            "The methods are in the group interfaces that this interface extends.",
        ],
    )?;
    let handlers: Vec<String> = members.iter().map(Member::handler).collect();
    writeln!(
        w,
        "export interface EffectHandler extends {} {{",
        handlers.join(", ")
    )?;
    w.indent();
    if let Some(render) = m.render_variant() {
        emit_method(w, render, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    // A method name comes from a variant name, so it never starts with an
    // underscore, and a field with one cannot clash with a method.
    writeln!(w)?;
    write_doc(
        w,
        &[
            "An `EffectHandler` made from one handler for each group. Each method",
            "forwards to the handler of its group.",
        ],
    )?;
    writeln!(
        w,
        "export class GroupedEffectHandler implements EffectHandler {{"
    )?;
    w.indent();
    for member in members {
        writeln!(
            w,
            "private readonly _{}: {};",
            member.parameter(),
            member.handler()
        )?;
    }
    writeln!(w)?;
    writeln!(w, "constructor(")?;
    w.indent();
    for member in members {
        writeln!(
            w,
            "{}: {},",
            param_name(&member.parameter()),
            member.handler()
        )?;
    }
    w.unindent();
    writeln!(w, ") {{")?;
    w.indent();
    for member in members {
        let parameter = member.parameter();
        writeln!(w, "this._{parameter} = {};", param_name(&parameter))?;
    }
    w.unindent();
    writeln!(w, "}}")?;
    for member in members {
        let target = format!("this._{}", member.parameter());
        for variant in &member.variants {
            writeln!(w)?;
            writeln!(w, "{} {{", signature(variant, config))?;
            w.indent();
            writeln!(w, "{}", forward(&target, variant))?;
            w.unindent();
            writeln!(w, "}}")?;
        }
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The statement that forwards `variant` to the same method on `target`.
fn forward(target: &str, variant: &Variant<'_>) -> String {
    let method = lower_camel(variant.name);
    if variant.is_legacy() {
        format!("{target}.{method}(operation, requestId, resolve);")
    } else if variant.is_stream() {
        format!("{target}.{method}(operation, sink);")
    } else if variant.is_request() {
        format!("return {target}.{method}(operation);")
    } else {
        format!("{target}.{method}(operation);")
    }
}

fn emit_dispatcher(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w)?;
    write_doc(
        w,
        &[
            "Routes each request to an `EffectHandler` and resolves it for you:",
            "never for a notification, once for a request, once per item for a",
            "stream.",
        ],
    )?;
    writeln!(w, "export class EffectDispatcher {{")?;
    w.indent();
    writeln!(w, "constructor(")?;
    w.indent();
    writeln!(w, "private readonly handler: EffectHandler,")?;
    writeln!(
        w,
        "private readonly resolve: (id: uint32, bytes: Uint8Array) => void,"
    )?;
    w.unindent();
    writeln!(w, ") {{}}")?;
    writeln!(w)?;
    writeln!(w, "public dispatch(request: Request): void {{")?;
    w.indent();
    writeln!(w, "const id = request.id;")?;
    writeln!(w, "const effect = request.effect;")?;
    writeln!(w, "switch (effect.kind) {{")?;
    w.indent();
    for variant in &m.variants {
        writeln!(w, r#"case "{}": {{"#, variant.name)?;
        w.indent();
        emit_arm(w, variant, &lower_camel(variant.name), config)?;
        writeln!(w, "break;")?;
        w.unindent();
        writeln!(w, "}}")?;
    }
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_arm(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    method: &str,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    if variant.is_legacy() {
        writeln!(
            w,
            "this.handler.{method}(effect.value, id, (bytes) => this.resolve(id, bytes));"
        )?;
    } else if variant.is_stream() {
        writeln!(w, "this.handler.{method}(effect.value, {{")?;
        w.indent();
        writeln!(w, "send: (item) => {{")?;
        w.indent();
        emit_resolve(w, "item", variant, config)?;
        w.unindent();
        writeln!(w, "}},")?;
        w.unindent();
        writeln!(w, "}});")?;
    } else if variant.is_request() {
        writeln!(
            w,
            "void this.handler.{method}(effect.value).then((output) => {{"
        )?;
        w.indent();
        emit_resolve(w, "output", variant, config)?;
        w.unindent();
        writeln!(w, "}});")?;
    } else if variant.render {
        writeln!(w, "this.handler.{method}?.(effect.value);")?;
    } else {
        writeln!(w, "this.handler.{method}(effect.value);")?;
    }

    Ok(())
}

fn emit_resolve(
    w: &mut dyn IndentWrite,
    value: &str,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w, "const serializer = new BincodeSerializer();")?;
    if let Some(output) = &variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    writeln!(w, "this.resolve(id, serializer.getBytes());")?;

    Ok(())
}

fn render_output(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant
        .output
        .as_ref()
        .map_or_else(|| "void".to_string(), |format| render_type(format, config))
}
