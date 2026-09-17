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

use super::{super::Matched, AppMeta, BoltFfi, render};

pub(super) fn emit(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
    ffi: Option<&BoltFfi>,
) -> io::Result<()> {
    emit_bridge(w)?;
    // The secondary constructor is only correct when `FfiBridge` is written
    // beside the module, which is what naming the Kotlin bindings asks for.
    let bridged = ffi.filter(|ffi| ffi.kotlin_ffi().is_some());
    emit_core(w, m, app, config, bridged)
}

/// `FfiBridge`, the `CoreBridge` over `BoltFFI`'s class, for its own file in the
/// generated package.
pub(super) fn ffi_bridge(class: &str, config: &CodeGeneratorConfig) -> io::Result<String> {
    render(config, |w| {
        writeln!(
            w,
            "/// The generated `CoreBridge`, implemented over BoltFFI's `{class}`: bytes"
        )?;
        writeln!(
            w,
            "/// in, bytes out, nothing else. It is the only generated type that knows"
        )?;
        writeln!(
            w,
            "/// the Rust core exists — pass a fake to `Core(bridge, handler, scope)`"
        )?;
        writeln!(w, "/// instead for tests.")?;
        writeln!(w, "///")?;
        writeln!(
            w,
            "/// `{class}` holds a handle to the Rust side, so closing an `FfiBridge`"
        )?;
        writeln!(w, "/// releases the core it wraps.")?;
        writeln!(
            w,
            "class FfiBridge(private val ffi: {class} = {class}()) : CoreBridge, AutoCloseable {{"
        )?;
        w.indent();
        writeln!(
            w,
            "override fun update(event: ByteArray): ByteArray = ffi.update(event)"
        )?;
        writeln!(w)?;
        writeln!(
            w,
            "override fun resolve(id: UInt, output: ByteArray): ByteArray = ffi.resolve(id, output)"
        )?;
        writeln!(w)?;
        writeln!(w, "override fun view(): ByteArray = ffi.view()")?;
        writeln!(w)?;
        writeln!(w, "override fun close() = ffi.close()")?;
        w.unindent();
        writeln!(w, "}}")
    })
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

/// The secondary constructor that builds the generated `FfiBridge`, so a shell
/// that uses the `BoltFFI` bindings never names a bridge at all.
fn emit_ffi_constructor(w: &mut dyn IndentWrite, class: &str) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// Build a `Core` over the `{class}` the Rust crate exports — the shell"
    )?;
    writeln!(w, "/// writes the handler and never sees the bridge.")?;
    writeln!(
        w,
        "constructor(handler: EffectHandler, scope: CoroutineScope) : this(FfiBridge(), handler, scope)"
    )
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

    if let Some(ffi) = ffi {
        emit_ffi_constructor(w, ffi.class_name())?;
    }

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
