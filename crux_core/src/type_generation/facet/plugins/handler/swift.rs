//! Swift: `EffectSink`, the `EffectHandler` protocol and `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::swift::write_serialize_value,
    indent::IndentWrite,
    swift::{case_name, render_type},
};

use super::Matched;

/// The generated package manifest does not declare platforms, so it defaults
/// to a deployment target older than Swift concurrency. Requests are dispatched
/// in a `Task`, so the handler API has to say when it is available.
const AVAILABILITY: &str = "@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)";

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
    writeln!(w, "public struct EffectSink<Item>: Sendable {{")?;
    w.indent();
    writeln!(w, "private let _send: @Sendable (Item) -> Void")?;
    writeln!(w)?;
    writeln!(
        w,
        "public init(_ send: @escaping @Sendable (Item) -> Void) {{"
    )?;
    w.indent();
    writeln!(w, "self._send = send")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "/// Send one item back to the core.")?;
    writeln!(w, "public func send(_ item: Item) {{")?;
    w.indent();
    writeln!(w, "self._send(item)")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_serialize_helper(w: &mut dyn IndentWrite) -> io::Result<()> {
    writeln!(w)?;
    writeln!(
        w,
        "/// Serializing an effect output cannot fail for any value the core can"
    )?;
    writeln!(
        w,
        "/// accept, so the dispatcher traps rather than making every handler"
    )?;
    writeln!(w, "/// method throw.")?;
    writeln!(
        w,
        "private func serializeEffectOutput(_ body: (BincodeSerializer) throws -> Void) -> [UInt8] {{"
    )?;
    w.indent();
    writeln!(w, "let serializer = BincodeSerializer.init()")?;
    writeln!(w, "try! body(serializer)")?;
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
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "public protocol EffectHandler: Sendable {{")?;
    w.indent();
    for variant in &m.variants {
        let method = case_name(variant.name);
        let operation = render_type(variant.operation, config);
        if variant.is_legacy() {
            writeln!(
                w,
                "func {method}(_ operation: {operation}, requestId: UInt32, resolve: @escaping @Sendable ([UInt8]) -> Void)"
            )?;
        } else if variant.is_stream() {
            let output = render_output(variant, config);
            writeln!(
                w,
                "func {method}(_ operation: {operation}, into sink: EffectSink<{output}>)"
            )?;
        } else if variant.is_request() {
            let output = render_output(variant, config);
            writeln!(
                w,
                "func {method}(_ operation: {operation}) async -> {output}"
            )?;
        } else {
            writeln!(w, "func {method}(_ operation: {operation})")?;
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
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "public struct EffectDispatcher: Sendable {{")?;
    w.indent();
    writeln!(w, "private let handler: any EffectHandler")?;
    writeln!(
        w,
        "private let resolve: @Sendable (UInt32, [UInt8]) -> Void"
    )?;
    writeln!(w)?;
    writeln!(
        w,
        "public init(handler: any EffectHandler, resolve: @escaping @Sendable (UInt32, [UInt8]) -> Void) {{"
    )?;
    w.indent();
    writeln!(w, "self.handler = handler")?;
    writeln!(w, "self.resolve = resolve")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "public func dispatch(_ request: Request) {{")?;
    w.indent();
    writeln!(w, "let handler = self.handler")?;
    writeln!(w, "let resolve = self.resolve")?;
    writeln!(w, "let id = request.id")?;
    writeln!(w, "switch request.effect {{")?;
    for variant in &m.variants {
        let case = case_name(variant.name);
        writeln!(w, "case .{case}(let operation):")?;
        w.indent();
        emit_arm(w, variant, &case, config)?;
        w.unindent();
    }
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

fn emit_arm(
    w: &mut dyn IndentWrite,
    variant: &super::super::Variant<'_>,
    method: &str,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    if variant.is_legacy() {
        writeln!(w, "handler.{method}(operation, requestId: id) {{ bytes in")?;
        w.indent();
        writeln!(w, "resolve(id, bytes)")?;
        w.unindent();
        writeln!(w, "}}")?;
    } else if variant.is_stream() {
        writeln!(w, "handler.{method}(operation, into: EffectSink {{ item in")?;
        w.indent();
        emit_resolve(w, "item", variant, config)?;
        w.unindent();
        writeln!(w, "}})")?;
    } else if variant.is_request() {
        writeln!(w, "Task {{")?;
        w.indent();
        writeln!(w, "let output = await handler.{method}(operation)")?;
        emit_resolve(w, "output", variant, config)?;
        w.unindent();
        writeln!(w, "}}")?;
    } else {
        writeln!(w, "handler.{method}(operation)")?;
    }

    Ok(())
}

fn emit_resolve(
    w: &mut dyn IndentWrite,
    value: &str,
    variant: &super::super::Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w, "resolve(id, serializeEffectOutput {{ serializer in")?;
    w.indent();
    if let Some(output) = variant.output {
        write_serialize_value(w, value, output, config)?;
    }
    w.unindent();
    writeln!(w, "}})")?;

    Ok(())
}

fn render_output(variant: &super::super::Variant<'_>, config: &CodeGeneratorConfig) -> String {
    variant
        .output
        .map_or_else(|| "Unit".to_string(), |format| render_type(format, config))
}
