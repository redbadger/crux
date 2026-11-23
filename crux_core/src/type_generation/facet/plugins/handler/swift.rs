//! Swift: `EffectSink`, the `EffectHandler` protocol and `EffectDispatcher`.

use std::io;

use facet_generate::generation::{
    CodeGeneratorConfig,
    bincode::swift::write_serialize_value,
    indent::IndentWrite,
    swift::{case_name, field_name, render_type, write_doc},
};

use super::{
    super::{Member, SWIFT_AVAILABILITY as AVAILABILITY, Variant},
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
    write_doc(w, &["Send one item back to the core."])?;
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
    write_doc(
        w,
        &[
            "Serializing an effect output cannot fail for any value the core can",
            "accept, so the dispatcher traps rather than making every handler",
            "method throw.",
        ],
    )?;
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
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "public protocol EffectHandler: Sendable {{")?;
    w.indent();
    for variant in &m.variants {
        emit_requirement(w, variant, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    emit_default_render(w, m, config)?;

    Ok(())
}

/// One requirement of a handler protocol. A requirement other than `render`
/// carries the doc comment of its operation.
fn emit_requirement(
    w: &mut dyn IndentWrite,
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    // `Core` handles the render operation, and its method has a doc of its
    // own, so only the other operations carry the operation's doc.
    if !variant.render {
        write_doc(w, variant.doc)?;
    }
    writeln!(w, "{}", signature(variant, config))
}

/// The signature of the handler method for `variant`.
fn signature(variant: &Variant<'_>, config: &CodeGeneratorConfig) -> String {
    let method = case_name(variant.name);
    let operation = render_type(variant.operation, config);
    if variant.is_legacy() {
        format!(
            "func {method}(_ operation: {operation}, requestId: UInt32, resolve: @escaping @Sendable ([UInt8]) -> Void)"
        )
    } else if variant.is_stream() {
        let output = render_output(variant, config);
        format!("func {method}(_ operation: {operation}, into sink: EffectSink<{output}>)")
    } else if variant.is_request() {
        let output = render_output(variant, config);
        format!("func {method}(_ operation: {operation}) async -> {output}")
    } else {
        format!("func {method}(_ operation: {operation})")
    }
}

/// The handler API when the app puts its effects into groups: one protocol
/// for each group, `EffectHandler` that refines them all, and
/// `GroupedEffectHandler` that forwards each method to the handler of its
/// group.
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
        writeln!(w, "{AVAILABILITY}")?;
        writeln!(w, "public protocol {}: Sendable {{", member.handler())?;
        w.indent();
        for variant in &member.variants {
            emit_requirement(w, variant, config)?;
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
            "The methods are in the group protocols that this protocol refines.",
        ],
    )?;
    writeln!(w, "{AVAILABILITY}")?;
    let handlers: Vec<String> = members.iter().map(Member::handler).collect();
    writeln!(
        w,
        "public protocol EffectHandler: Sendable, {} {{",
        handlers.join(", ")
    )?;
    w.indent();
    if let Some(render) = m.render_variant() {
        emit_requirement(w, render, config)?;
    }
    w.unindent();
    writeln!(w, "}}")?;

    emit_default_render(w, m, config)?;

    writeln!(w)?;
    write_doc(
        w,
        &[
            "An `EffectHandler` made from one handler for each group. Each method",
            "forwards to the handler of its group.",
        ],
    )?;
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "public struct GroupedEffectHandler: EffectHandler {{")?;
    w.indent();
    for member in members {
        let parameter = field_name(&member.parameter());
        writeln!(w, "public let {parameter}: any {}", member.handler())?;
    }
    writeln!(w)?;
    let parameters: Vec<String> = members
        .iter()
        .map(|member| {
            format!(
                "{}: any {}",
                field_name(&member.parameter()),
                member.handler()
            )
        })
        .collect();
    writeln!(w, "public init({}) {{", parameters.join(", "))?;
    w.indent();
    for member in members {
        let parameter = field_name(&member.parameter());
        writeln!(w, "self.{parameter} = {parameter}")?;
    }
    w.unindent();
    writeln!(w, "}}")?;
    for member in members {
        let target = format!("self.{}", field_name(&member.parameter()));
        for variant in &member.variants {
            writeln!(w)?;
            writeln!(w, "public {} {{", signature(variant, config))?;
            w.indent();
            writeln!(w, "{}", forward(&target, variant))?;
            w.unindent();
            writeln!(w, "}}")?;
        }
    }
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The call that forwards `variant` to the same method on `target`.
fn forward(target: &str, variant: &Variant<'_>) -> String {
    let method = case_name(variant.name);
    if variant.is_legacy() {
        format!("{target}.{method}(operation, requestId: requestId, resolve: resolve)")
    } else if variant.is_stream() {
        format!("{target}.{method}(operation, into: sink)")
    } else if variant.is_request() {
        format!("await {target}.{method}(operation)")
    } else {
        format!("{target}.{method}(operation)")
    }
}

/// `Core` handles the render variant itself, so a handler written for it never
/// implements that method.
fn emit_default_render(
    w: &mut dyn IndentWrite,
    m: &Matched<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    let Some(variant) = m.render_variant() else {
        return Ok(());
    };
    let method = case_name(variant.name);
    let operation = render_type(variant.operation, config);

    writeln!(w)?;
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
    writeln!(w, "{AVAILABILITY}")?;
    writeln!(w, "extension EffectHandler {{")?;
    w.indent();
    writeln!(w, "public func {method}(_ operation: {operation}) {{}}")?;
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
    write_doc(
        w,
        &[
            "Routes each request to an `EffectHandler` and resolves it for you:",
            "never for a notification, once for a request, once per item for a",
            "stream.",
        ],
    )?;
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
    variant: &Variant<'_>,
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
    variant: &Variant<'_>,
    config: &CodeGeneratorConfig,
) -> io::Result<()> {
    writeln!(w, "resolve(id, serializeEffectOutput {{ serializer in")?;
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
