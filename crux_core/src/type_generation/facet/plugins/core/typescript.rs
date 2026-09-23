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

use super::{super::Matched, AppMeta, BoltFfi};

/// `app` names the event and view model in the emitter's spelling, as the
/// formats in `m` are.
pub(super) fn emit(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
    ffi: Option<&BoltFfi>,
) -> io::Result<()> {
    emit_bridge(w)?;
    // TypeScript generates one file per module, so the bridge goes in here
    // rather than into a companion file.
    let bridged = ffi.filter(|ffi| ffi.typescript_ffi().is_some());
    if let Some(ffi) = bridged {
        emit_ffi_bridge(w, ffi.class_name())?;
    }
    emit_core(w, m, app, config, bridged)
}

/// `FfiBridge`, the `CoreBridge` over `BoltFFI`'s wasm bindings.
fn emit_ffi_bridge(w: &mut dyn IndentWrite, class: &str) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// The generated `CoreBridge`, implemented over BoltFFI's `{class}`: bytes"
    )?;
    writeln!(
        w,
        "/// in, bytes out, nothing else. `{class}.new()` reaches into the wasm"
    )?;
    writeln!(
        w,
        "/// module, so build one only once the package has loaded — `Core.create`"
    )?;
    writeln!(w, "/// waits for that.")?;
    writeln!(w, "export class FfiBridge implements CoreBridge {{")?;
    w.indent();
    writeln!(w, "private readonly ffi = boltffi.{class}.new();")?;
    writeln!(w)?;
    writeln!(w, "update(event: Uint8Array): Uint8Array {{")?;
    w.indent();
    writeln!(w, "return this.ffi.update(event);")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "resolve(id: uint32, output: Uint8Array): Uint8Array {{")?;
    w.indent();
    writeln!(w, "return this.ffi.resolve(id, output);")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "view(): Uint8Array {{")?;
    w.indent();
    writeln!(w, "return this.ffi.view();")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")
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
    ffi: Option<&BoltFfi>,
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

    if let Some(ffi) = ffi {
        emit_create(w, ffi.class_name(), &view_model)?;
    }

    emit_methods(w, app, config, &event, &view_model, render.name)?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The one-argument constructor, as a static factory: the wasm module has to
/// finish loading before `CoreFfi.new()` can be called, and a constructor
/// cannot await.
fn emit_create(w: &mut dyn IndentWrite, class: &str, view_model: &str) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// Build a `Core` over the `{class}` the Rust crate exports, once the"
    )?;
    writeln!(w, "/// wasm module has loaded.")?;
    writeln!(w, "public static async create(")?;
    w.indent();
    writeln!(w, "handler: EffectHandler,")?;
    writeln!(w, "onView: (view: {view_model}) => void,")?;
    w.unindent();
    writeln!(w, "): Promise<Core> {{")?;
    w.indent();
    // `initialized` is not in the package's type declarations, so it has to be
    // reached for explicitly.
    writeln!(
        w,
        "await (boltffi as unknown as {{ initialized: Promise<void> }}).initialized;"
    )?;
    writeln!(w, "return new Core(new FfiBridge(), handler, onView);")?;
    w.unindent();
    writeln!(w, "}}")
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
/// enum has a free function, reached through its namespace's import like the
/// type itself, and everything else a static method. `name` is in the
/// emitter's spelling, which is what the enum lookup is keyed by.
fn deserialize_expr(name: &QualifiedTypeName, config: &CodeGeneratorConfig) -> String {
    if config.is_enum(name) {
        let function = QualifiedTypeName {
            namespace: name.namespace.clone(),
            name: format!("deserialize{}", name.name),
        };
        let function = function.format(ToUpperCamelCase::to_upper_camel_case, ".");
        format!("{function}(deserializer)")
    } else {
        let type_name = name.format(ToUpperCamelCase::to_upper_camel_case, ".");
        format!("{type_name}.deserialize(deserializer)")
    }
}
