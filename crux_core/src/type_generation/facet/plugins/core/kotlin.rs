//! Kotlin: the `CoreBridge` interface and the `Core` class.

use std::io;

use facet_generate::{
    generation::{
        CodeGeneratorConfig,
        indent::IndentWrite,
        kotlin::{render_type, variant_class_name},
    },
    reflection::format::Format,
};

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
        "/// The core's FFI surface, as bytes. Implement it around the `CoreFfi` your"
    )?;
    writeln!(w, "/// Rust crate exports, or with a fake for tests.")?;
    writeln!(w, "interface CoreBridge {{")?;
    w.indent();
    writeln!(w, "fun update(event: ByteArray): ByteArray")?;
    writeln!(w, "fun resolve(id: UInt, output: ByteArray): ByteArray")?;
    writeln!(w, "fun view(): ByteArray")?;
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
    let render_class = variant_class_name(render.name);
    let effect = m.name;
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
    writeln!(
        w,
        "/// Each request is dispatched in its own coroutine on `scope`."
    )?;
    writeln!(w, "class Core(")?;
    w.indent();
    writeln!(w, "private val bridge: CoreBridge,")?;
    writeln!(w, "handler: EffectHandler,")?;
    writeln!(w, "private val scope: CoroutineScope,")?;
    w.unindent();
    writeln!(w, ") {{")?;
    w.indent();
    writeln!(
        w,
        "private val _view = MutableStateFlow({view_model}.bincodeDeserialize(bridge.view()))"
    )?;
    writeln!(w, "/// The view model as of the last `{}`.", render.name)?;
    writeln!(w, "val view: StateFlow<{view_model}> = _view.asStateFlow()")?;
    writeln!(
        w,
        "private val dispatcher = EffectDispatcher(handler) {{ id, bytes ->"
    )?;
    w.indent();
    writeln!(w, "scope.launch {{ process(bridge.resolve(id, bytes)) }}")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "/// Send an event to the core and run the loop it starts."
    )?;
    writeln!(w, "fun update(event: {event}) {{")?;
    w.indent();
    writeln!(w, "process(bridge.update(event.bincodeSerialize()))")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "/// Serialized requests, as returned by the bridge or pushed by middleware."
    )?;
    writeln!(w, "fun process(bytes: ByteArray) {{")?;
    w.indent();
    writeln!(w, "if (bytes.isEmpty()) return")?;
    writeln!(w, "process(Requests.bincodeDeserialize(bytes).value)")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "fun process(requests: List<Request>) {{")?;
    w.indent();
    writeln!(w, "for (request in requests) {{")?;
    w.indent();
    writeln!(w, "if (request.effect is {effect}.{render_class}) {{")?;
    w.indent();
    writeln!(
        w,
        "_view.value = {view_model}.bincodeDeserialize(bridge.view())"
    )?;
    writeln!(w, "continue")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "scope.launch {{ dispatcher.dispatch(request) }}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
