//! TypeScript: the `CoreBridge` interface and the `Core` class.

use std::io;

use facet_generate::{
    generation::{
        CodeGeneratorConfig, bincode::typescript::write_serialize_value, indent::IndentWrite,
        typescript::render_type,
    },
    reflection::format::{Format, QualifiedTypeName},
};
use heck::ToUpperCamelCase;

use super::{super::Matched, AppMeta};

pub(super) fn emit(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    emit_bridge(w)?;
    emit_core(w, m, app, config)
}

fn emit_bridge(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// The core's FFI surface, as bytes. Implement it around the bindings your"
    )?;
    writeln!(w, "/// Rust crate exports, or with a fake for tests.")?;
    writeln!(w, "export interface CoreBridge {{")?;
    w.indent();
    writeln!(w, "update(event: Uint8Array): Uint8Array;")?;
    writeln!(w, "resolve(id: uint32, output: Uint8Array): Uint8Array;")?;
    writeln!(w, "view(): Uint8Array;")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_core(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let render = m
        .render_variant()
        .expect("the plugin only matches an effect with a render variant");
    let event = render_type(&Format::TypeName(app.event.clone()), config);
    let view_model = render_type(&Format::TypeName(app.view_model.clone()), config);

    writeln!(w)?;
    writeln!(
        w,
        "/// The shell's side of the core: it serializes events, dispatches the"
    )?;
    writeln!(
        w,
        "/// requests that come back, and resolves them through the bridge."
    )?;
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// `{}` is handled here, so an `EffectHandler` need not implement it.",
        render.name
    )?;
    writeln!(w, "export class Core {{")?;
    w.indent();
    writeln!(w, "/// The view model as of the last `{}`.", render.name)?;
    writeln!(w, "public view: {view_model};")?;
    writeln!(w, "private readonly dispatcher: EffectDispatcher;")?;
    writeln!(w)?;
    writeln!(w, "constructor(")?;
    w.indent();
    writeln!(w, "private readonly bridge: CoreBridge,")?;
    writeln!(w, "handler: EffectHandler,")?;
    writeln!(w, "private readonly onView: (view: {view_model}) => void,")?;
    w.unindent();
    writeln!(w, ") {{")?;
    w.indent();
    writeln!(w, "this.view = this.readView();")?;
    writeln!(
        w,
        "this.dispatcher = new EffectDispatcher(handler, (id, bytes) => {{"
    )?;
    w.indent();
    writeln!(w, "this.processBytes(this.bridge.resolve(id, bytes));")?;
    w.unindent();
    writeln!(w, "}});")?;
    w.unindent();
    writeln!(w, "}}")?;

    emit_methods(w, app, config, &event, &view_model, render.name)?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_methods(
    w: &mut dyn IndentWrite,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
    event: &str,
    view_model: &str,
    render: &str,
) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// Send an event to the core and run the loop it starts."
    )?;
    writeln!(w, "public update(event: {event}): void {{")?;
    w.indent();
    writeln!(w, "const serializer = new BincodeSerializer();")?;
    write_serialize_value(w, "event", &Format::TypeName(app.event.clone()), config)?;
    writeln!(
        w,
        "this.processBytes(this.bridge.update(serializer.getBytes()));"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "/// Serialized requests, as returned by the bridge or pushed by middleware."
    )?;
    writeln!(w, "public processBytes(bytes: Uint8Array): void {{")?;
    w.indent();
    writeln!(w, "if (bytes.length === 0) {{ return; }}")?;
    writeln!(w, "const deserializer = new BincodeDeserializer(bytes);")?;
    writeln!(w, "this.process(Requests.deserialize(deserializer).value);")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "public process(requests: Request[]): void {{")?;
    w.indent();
    writeln!(w, "for (const request of requests) {{")?;
    w.indent();
    writeln!(w, r#"if (request.effect.kind === "{render}") {{"#)?;
    w.indent();
    writeln!(w, "this.view = this.readView();")?;
    writeln!(w, "this.onView(this.view);")?;
    writeln!(w, "continue;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "this.dispatcher.dispatch(request);")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "private readView(): {view_model} {{")?;
    w.indent();
    writeln!(
        w,
        "const deserializer = new BincodeDeserializer(this.bridge.view());"
    )?;
    writeln!(w, "return {};", deserialize_expr(&app.view_model, config))?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// How the bincode emitter spells "read one of these from `deserializer`": an
/// enum has a free function, everything else a static method.
fn deserialize_expr(name: &QualifiedTypeName, config: &CodeGeneratorConfig) -> String {
    let type_name = name.format(ToUpperCamelCase::to_upper_camel_case, ".");
    if config.enum_type_names.contains(&type_name) {
        format!("deserialize{type_name}(deserializer)")
    } else {
        format!("{type_name}.deserialize(deserializer)")
    }
}
