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
    super::{Matched, SWIFT_OBSERVABLE_AVAILABILITY as AVAILABILITY},
    AppMeta, BoltFfi, render,
};
use crate::type_generation::facet::SwiftFfi;

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
    // The convenience initializer is only correct when `FfiBridge` is written
    // beside the module, which is what naming the Swift bindings asks for.
    let bridged = ffi.filter(|ffi| ffi.swift_ffi().is_some());
    emit_core(w, m, app, config, bridged)
}

/// The SPM package name a `.product(..)` has to name: SPM takes it from the
/// last component of the path, not from anything inside the manifest.
pub(super) fn package_name(package_path: &str) -> &str {
    package_path
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(package_path)
}

/// `FfiBridge`, the `CoreBridge` over `BoltFFI`'s class, for its own file beside
/// the generated module.
pub(super) fn ffi_bridge(
    swift: &SwiftFfi,
    class: &str,
    config: &CodeGeneratorConfig,
) -> io::Result<String> {
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
            "/// the Rust core exists — pass a fake to `Core(bridge:handler:)` instead"
        )?;
        writeln!(w, "/// for previews and tests.")?;
        writeln!(w, "///")?;
        writeln!(
            w,
            "/// `@unchecked Sendable` because `{class}` is a class Swift cannot prove"
        )?;
        writeln!(
            w,
            "/// `Sendable`; its state is a handle into a mutex-guarded bridge on the"
        )?;
        writeln!(w, "/// Rust side, so calling it from any task is safe.")?;
        writeln!(
            w,
            "public struct FfiBridge: CoreBridge, @unchecked Sendable {{"
        )?;
        w.indent();
        // Qualified by module, so a registered type called `CoreFfi` cannot
        // shadow the one the bindings declare.
        writeln!(w, "private let ffi = {}.{class}()", swift.module)?;
        writeln!(w)?;
        writeln!(w, "public init() {{}}")?;
        writeln!(w)?;
        writeln!(w, "public func update(_ event: [UInt8]) -> [UInt8] {{")?;
        w.indent();
        writeln!(w, "[UInt8](ffi.update(data: Data(event)))")?;
        w.unindent();
        writeln!(w, "}}")?;
        writeln!(w)?;
        writeln!(
            w,
            "public func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8] {{"
        )?;
        w.indent();
        writeln!(w, "[UInt8](ffi.resolve(id: id, data: Data(output)))")?;
        w.unindent();
        writeln!(w, "}}")?;
        writeln!(w)?;
        writeln!(w, "public func view() -> [UInt8] {{")?;
        w.indent();
        writeln!(w, "[UInt8](ffi.view())")?;
        w.unindent();
        writeln!(w, "}}")?;
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
    ffi: Option<&BoltFfi>,
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
    writeln!(w, "///")?;
    writeln!(
        w,
        "/// `view` is observable, so a SwiftUI view that reads it is invalidated"
    )?;
    writeln!(w, "/// whenever the core renders.")?;
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "@Observable")?;
    writeln!(w, "@MainActor")?;
    writeln!(w, "public final class Core {{")?;
    w.indent();
    writeln!(w, "/// The view model as of the last `{}`.", render.name)?;
    writeln!(w, "public private(set) var view: {view_model}")?;
    writeln!(w, "@ObservationIgnored private let bridge: any CoreBridge")?;
    // The resolve callback captures `self`, which Swift will not allow until
    // every stored property is initialized — hence the implicitly unwrapped
    // optional.
    writeln!(
        w,
        "@ObservationIgnored private var dispatcher: EffectDispatcher!"
    )?;
    writeln!(w)?;
    writeln!(
        w,
        "public init(bridge: any CoreBridge, handler: any EffectHandler) {{"
    )?;
    w.indent();
    writeln!(w, "self.bridge = bridge")?;
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

    if let Some(ffi) = ffi {
        writeln!(w)?;
        writeln!(
            w,
            "/// Build a `Core` over the `{}` the Rust crate exports — the shell",
            ffi.class_name()
        )?;
        writeln!(w, "/// writes the handler and never sees the bridge.")?;
        writeln!(w, "public convenience init(handler: any EffectHandler) {{")?;
        w.indent();
        writeln!(w, "self.init(bridge: FfiBridge(), handler: handler)")?;
        w.unindent();
        writeln!(w, "}}")?;
    }

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
