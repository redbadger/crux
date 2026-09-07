//! Swift: the `CoreBridge` protocol and the `Core` class.

use std::io;

use facet_generate::{
    generation::{
        CodeGeneratorConfig,
        indent::IndentWrite,
        swift::{case_name, render_type},
    },
    reflection::format::Format,
};

use super::{
    super::{Matched, SWIFT_AVAILABILITY as AVAILABILITY},
    AppMeta,
};

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
    writeln!(
        w,
        "/// Rust crate exports, or with a fake for previews and tests."
    )?;
    writeln!(w, "public protocol CoreBridge: Sendable {{")?;
    w.indent();
    writeln!(w, "func update(_ event: [UInt8]) -> [UInt8]")?;
    writeln!(
        w,
        "func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8]"
    )?;
    writeln!(w, "func view() -> [UInt8]")?;
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
    let render_case = case_name(render.name);
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
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "@MainActor")?;
    writeln!(w, "public final class Core {{")?;
    w.indent();
    writeln!(w, "/// The view model as of the last `{}`.", render.name)?;
    writeln!(w, "public private(set) var view: {view_model}")?;
    writeln!(w, "private let bridge: any CoreBridge")?;
    writeln!(w, "private let onView: @MainActor ({view_model}) -> Void")?;
    // The resolve callback captures `self`, which Swift will not allow until
    // every stored property is initialized — hence the implicitly unwrapped
    // optional.
    writeln!(w, "private var dispatcher: EffectDispatcher!")?;
    writeln!(w)?;
    writeln!(
        w,
        "public init(bridge: any CoreBridge, handler: any EffectHandler, onView: @escaping @MainActor ({view_model}) -> Void) {{"
    )?;
    w.indent();
    writeln!(w, "self.bridge = bridge")?;
    writeln!(w, "self.onView = onView")?;
    writeln!(
        w,
        "self.view = try! {view_model}.bincodeDeserialize(input: bridge.view())"
    )?;
    writeln!(
        w,
        "self.dispatcher = EffectDispatcher(handler: handler) {{ [weak self] id, bytes in"
    )?;
    w.indent();
    writeln!(w, "Task {{ @MainActor in")?;
    w.indent();
    writeln!(w, "guard let self else {{ return }}")?;
    writeln!(w, "self.process(bytes: self.bridge.resolve(id, bytes))")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    emit_methods(w, &event, &view_model, &render_case)?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_methods(
    w: &mut dyn IndentWrite,
    event: &str,
    view_model: &str,
    render_case: &str,
) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// Send an event to the core and run the loop it starts."
    )?;
    writeln!(w, "public func update(_ event: {event}) {{")?;
    w.indent();
    writeln!(
        w,
        "process(bytes: bridge.update(try! event.bincodeSerialize()))"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "/// Serialized requests, as returned by the bridge or pushed by middleware."
    )?;
    writeln!(w, "public func process(bytes: [UInt8]) {{")?;
    w.indent();
    writeln!(w, "if bytes.isEmpty {{ return }}")?;
    writeln!(
        w,
        "process(try! Requests.bincodeDeserialize(input: bytes).value)"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "public func process(_ requests: [Request]) {{")?;
    w.indent();
    writeln!(w, "for request in requests {{")?;
    w.indent();
    writeln!(w, "if case .{render_case} = request.effect {{")?;
    w.indent();
    writeln!(
        w,
        "view = try! {view_model}.bincodeDeserialize(input: bridge.view())"
    )?;
    writeln!(w, "onView(view)")?;
    writeln!(w, "continue")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "dispatcher.dispatch(request)")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}
