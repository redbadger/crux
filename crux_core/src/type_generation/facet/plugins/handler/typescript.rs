//! TypeScript: `EffectSink`, the `EffectHandler` interface and
//! `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig, bincode::typescript::write_serialize_value, indent::IndentWrite,
    typescript::render_type,
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
    writeln!(w, "/// Receives the items of a streaming effect.")?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// Every item sent is one resolution of the request that started the stream."
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
    writeln!(w, "export interface EffectHandler {{")?;
    w.indent();
    for variant in &m.variants {
        let method = lower_camel(variant.name);
        let operation = render_type(variant.operation, config);
        if variant.is_legacy() {
            writeln!(
                w,
                "{method}(operation: {operation}, requestId: uint32, resolve: (bytes: Uint8Array) => void): void;"
            )?;
        } else if variant.is_stream() {
            let output = render_output(variant, config);
            writeln!(
                w,
                "{method}(operation: {operation}, sink: EffectSink<{output}>): void;"
            )?;
        } else if variant.is_request() {
            let output = render_output(variant, config);
            writeln!(w, "{method}(operation: {operation}): Promise<{output}>;")?;
        } else {
            writeln!(w, "{method}(operation: {operation}): void;")?;
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
    if let Some(output) = variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    writeln!(w, "this.resolve(id, serializer.getBytes());")?;

    Ok(())
}

fn render_output(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant
        .output
        .map_or_else(|| "void".to_string(), |format| render_type(format, config))
}
