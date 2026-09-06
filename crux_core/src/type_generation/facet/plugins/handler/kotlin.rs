//! Kotlin: `EffectSink`, the `EffectHandler` interface and
//! `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::kotlin::write_serialize_value,
    indent::IndentWrite,
    kotlin::{render_type, variant_class_name},
};

use super::{super::Variant, super::lower_camel, Matched};

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
    writeln!(w, "/// Receives the items of a streaming effect.")?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// Every item sent is one resolution of the request that started the stream."
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
    writeln!(w)?;
    writeln!(w, "/// Handles every effect `{}` can carry.", m.name)?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// `EffectDispatcher` resolves each request for you, so there is no"
    )?;
    writeln!(
        w,
        "/// `resolve` to call at the wrong time or with the wrong bytes."
    )?;
    writeln!(w, "interface EffectHandler {{")?;
    w.indent();
    for variant in &m.variants {
        let method = lower_camel(variant.name);
        let operation = render_type(variant.operation, config);
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
        } else {
            writeln!(w, "fun {method}(operation: {operation})")?;
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
    let effect = m.name;

    writeln!(w)?;
    writeln!(
        w,
        "/// Routes each request to an `EffectHandler` and resolves it for you:"
    )?;
    writeln!(
        w,
        "/// never for a notification, once for a request, once per item for a"
    )?;
    writeln!(w, "/// stream.")?;
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
    if let Some(output) = variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    w.unindent();
    writeln!(w, "}})")?;

    Ok(())
}

fn render_output(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant
        .output
        .map_or_else(|| "Unit".to_string(), |format| render_type(format, config))
}
