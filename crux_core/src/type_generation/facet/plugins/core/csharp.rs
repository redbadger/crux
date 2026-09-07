//! C#: the `ICoreBridge` interface and the `Core` class.

use std::io;

use facet_generate::{
    generation::{
        CodeGeneratorConfig,
        bincode::csharp::write_serialize_value,
        csharp::{escape_identifier, render_type},
        indent::IndentWrite,
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
    let event = escape_identifier("event");

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// The core's FFI surface, as bytes. Implement it around the bindings your"
    )?;
    writeln!(w, "/// Rust crate exports, or with a fake for tests.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public interface ICoreBridge")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "byte[] Update(byte[] {event});")?;
    writeln!(w, "byte[] Resolve(uint id, byte[] output);")?;
    writeln!(w, "byte[] View();")?;
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
    let ns = &config.module_name;
    let effect = m.name;
    let render = m
        .render_variant()
        .expect("the plugin only matches an effect with a render variant");
    let event_param = escape_identifier("event");
    let event = render_type(&Format::TypeName(app.event.clone()), config);
    let view_model = render_type(&Format::TypeName(app.view_model.clone()), config);
    // A C-style enum keeps its bincode entry points in a companion static
    // class, exactly as the bincode emitter spells it.
    let view_model_bincode = if config.unit_variant_enums.contains(&app.view_model.name) {
        format!("{view_model}Bincode")
    } else {
        view_model.clone()
    };

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
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
        "/// <c>{}</c> is handled here, so an <c>IEffectHandler</c> need not",
        render.name
    )?;
    writeln!(
        w,
        "/// implement it. <c>onView</c> may be called on a thread-pool thread after"
    )?;
    writeln!(
        w,
        "/// an asynchronous request, so marshal to your UI thread inside it."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public sealed class Core")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "private readonly ICoreBridge _bridge;")?;
    writeln!(w, "private readonly EffectDispatcher _dispatcher;")?;
    writeln!(w, "private readonly Action<{view_model}> _onView;")?;
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// The view model as of the last <c>{}</c>.",
        render.name
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public {view_model} View {{ get; private set; }}")?;
    writeln!(w)?;
    writeln!(
        w,
        "public Core(ICoreBridge bridge, IEffectHandler handler, Action<{view_model}> onView)"
    )?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_bridge = bridge;")?;
    writeln!(w, "_onView = onView;")?;
    writeln!(w, "View = ReadView();")?;
    writeln!(
        w,
        "_dispatcher = new EffectDispatcher(handler, (id, bytes) => Process(_bridge.Resolve(id, bytes)));"
    )?;
    w.unindent();
    writeln!(w, "}}")?;

    emit_methods(
        w,
        app,
        config,
        &Names {
            ns,
            effect,
            render: render.name,
            event: &event,
            event_param: &event_param,
            view_model: &view_model,
            view_model_bincode: &view_model_bincode,
        },
    )?;
    w.unindent();
    writeln!(w, "}}")?;

    Ok(())
}

/// The names `Core`'s methods are spelled with, worked out once in
/// [`emit_core`].
struct Names<'a> {
    ns: &'a str,
    effect: &'a str,
    render: &'a str,
    event: &'a str,
    event_param: &'a str,
    view_model: &'a str,
    view_model_bincode: &'a str,
}

fn emit_methods(
    w: &mut dyn IndentWrite,
    app: &AppMeta,
    config: &CodeGeneratorConfig,
    names: &Names<'_>,
) -> io::Result<()> {
    let &Names {
        ns,
        effect,
        render,
        event,
        event_param,
        view_model,
        view_model_bincode,
    } = names;

    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Send an event to the core and run the loop it starts."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public void Update({event} {event_param})")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "var serializer = new BincodeSerializer();")?;
    write_serialize_value(w, event_param, &Format::TypeName(app.event.clone()), config)?;
    writeln!(w, "Process(_bridge.Update(serializer.GetBytes()));")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(
        w,
        "/// Serialized requests, as returned by the bridge or pushed by middleware."
    )?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public void Process(byte[] bytes)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "if (bytes.Length == 0) {{ return; }}")?;
    writeln!(w, "Process({ns}.Requests.BincodeDeserialize(bytes).Value);")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "public void Process(IReadOnlyList<{ns}.Request> requests)"
    )?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "foreach (var request in requests)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "if (request.Effect is {ns}.{effect}.{render})")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "View = ReadView();")?;
    writeln!(w, "_onView(View);")?;
    writeln!(w, "continue;")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w, "_dispatcher.Dispatch(request);")?;
    w.unindent();
    writeln!(w, "}}")?;
    w.unindent();
    writeln!(w, "}}")?;
    writeln!(w)?;
    writeln!(
        w,
        "private {view_model} ReadView() => {view_model_bincode}.BincodeDeserialize(_bridge.View());"
    )?;

    Ok(())
}
