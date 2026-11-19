//! Kotlin: `EffectSink`, the `EffectHandler` interface and
//! `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::kotlin::write_serialize_value,
    indent::IndentWrite,
    kotlin::{escape_identifier, render_type, variant_class_name, write_doc},
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
    emit_serialize_helper(w)?;
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
    writeln!(w, "fun interface EffectSink<in T> {{")?;
    w.indent();
    writeln!(w, "fun send(item: T)")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_serialize_helper(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "private fun effectOutputBytes(body: (Serializer) -> Unit): ByteArray {{"
    )?;
    w.indent();
    writeln!(w, "val serializer = BincodeSerializer()")?;
    writeln!(w, "body(serializer)")?;
    writeln!(w, "return serializer.get_bytes()")?;
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
    writeln!(w, "interface EffectHandler {{")?;
    w.indent();
    for variant in &m.variants {
        emit_method(w, variant, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// One method of a handler interface. A method other than `render` carries
/// the doc comment of its operation, and `render` does nothing by default.
fn emit_method(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let method = lower_camel(variant.name);
    let operation = render_type(variant.operation, config);
    // `Core` handles the render operation, and its method has a doc of its
    // own, so only the other operations carry the operation's doc.
    if !variant.render {
        write_doc(w, variant.doc)?;
    }
    if variant.is_legacy() {
        writeln!(
            w,
            "fun {method}(operation: {operation}, requestId: UInt, resolve: (ByteArray) -> Unit)"
        )?;
    } else if variant.is_stream() {
        let output = render_output(variant, config);
        writeln!(
            w,
            "fun {method}(operation: {operation}, sink: EffectSink<{output}>)"
        )?;
    } else if variant.is_request() {
        let output = render_output(variant, config);
        writeln!(w, "suspend fun {method}(operation: {operation}): {output}")?;
    } else if variant.render {
        write_doc(
            w,
            &[
                &format!(
                    "`Core` handles `{}` itself, so this does nothing; implement it",
                    variant.name
                ),
                "only if you drive `EffectDispatcher` yourself.",
            ],
        )?;
        writeln!(w, "fun {method}(operation: {operation}) {{}}")?;
    } else {
        writeln!(w, "fun {method}(operation: {operation})")?;
    }

    Ok(())
}

/// The handler API when the app puts its effects into groups: one interface
/// for each group, `EffectHandler` that extends them all, and
/// `GroupedEffectHandler`, which forwards each method to the handler of its
/// group with Kotlin delegation.
///
/// Delegation works because `EffectHandler` does not declare the methods of
/// the groups again. If it did, each method would come from two
/// supertypes, and Kotlin would ask for an explicit override.
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
        writeln!(w, "interface {} {{", member.handler())?;
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
    writeln!(w, "interface EffectHandler : {} {{", handlers.join(", "))?;
    w.indent();
    if let Some(render) = m.render_variant() {
        emit_method(w, render, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    writeln!(w)?;
    write_doc(
        w,
        &[
            "An `EffectHandler` made from one handler for each group. Each method",
            "forwards to the handler of its group.",
        ],
    )?;
    writeln!(w, "class GroupedEffectHandler(")?;
    w.indent();
    for member in members {
        let parameter = escape_identifier(&member.parameter()).into_owned();
        writeln!(w, "{parameter}: {},", member.handler())?;
    }
    w.unindent();
    let delegates: Vec<String> = members
        .iter()
        .map(|member| {
            format!(
                "{} by {}",
                member.handler(),
                escape_identifier(&member.parameter())
            )
        })
        .collect();
    writeln!(w, ") : EffectHandler, {}", delegates.join(", "))?;

    Ok(())
}

fn emit_dispatcher(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let effect = m.name;

    writeln!(w)?;
    write_doc(
        w,
        &[
            "Routes each request to an `EffectHandler` and resolves it for you:",
            "never for a notification, once for a request, once per item for a",
            "stream.",
        ],
    )?;
    writeln!(w, "class EffectDispatcher(")?;
    w.indent();
    writeln!(w, "private val handler: EffectHandler,")?;
    writeln!(w, "private val resolve: (UInt, ByteArray) -> Unit,")?;
    w.unindent();
    writeln!(w, ") {{")?;
    w.indent();
    writeln!(w, "suspend fun dispatch(request: Request) {{")?;
    w.indent();
    writeln!(w, "val id = request.id")?;
    writeln!(w, "when (val effect = request.effect) {{")?;
    w.indent();
    for variant in &m.variants {
        let class = variant_class_name(variant.name);
        write!(w, "is {effect}.{class} -> ")?;
        emit_arm(w, variant, &lower_camel(variant.name), config)?;
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
            "handler.{method}(effect.value, id) {{ bytes -> resolve(id, bytes) }}"
        )?;
    } else if variant.is_stream() {
        writeln!(w, "handler.{method}(effect.value) {{ item ->")?;
        w.indent();
        emit_resolve(w, "item", variant, config)?;
        w.unindent();
        writeln!(w, "}}")?;
    } else if variant.is_request() {
        writeln!(w, "{{")?;
        w.indent();
        writeln!(w, "val output = handler.{method}(effect.value)")?;
        emit_resolve(w, "output", variant, config)?;
        w.unindent();
        writeln!(w, "}}")?;
    } else {
        writeln!(w, "handler.{method}(effect.value)")?;
    }

    Ok(())
}

fn emit_resolve(
    w: &mut dyn IndentWrite,
    value: &str,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w, "resolve(id, effectOutputBytes {{ serializer ->")?;
    w.indent();
    if let Some(output) = &variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    w.unindent();
    writeln!(w, "}})")?;

    Ok(())
}

fn render_output(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant
        .output
        .as_ref()
        .map_or_else(|| "Unit".to_string(), |format| render_type(format, config))
}
