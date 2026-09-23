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

use super::{super::Matched, AppMeta, BoltFfi, render};

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
    // The one-argument constructor is only correct when `FfiBridge` is written
    // beside the module, which is what naming the C# bindings asks for.
    let bridged = ffi.filter(|ffi| ffi.csharp_ffi().is_some());
    emit_core(w, m, app, config, bridged)
}

/// `FfiBridge`, the `ICoreBridge` over `BoltFFI`'s class, for its own file in the
/// generated namespace.
pub(super) fn ffi_bridge(class: &str, config: &CodeGeneratorConfig) -> io::Result<String> {
    let event = escape_identifier("event");

    render(config, |w| {
        writeln!(w, "/// <summary>")?;
        writeln!(
            w,
            "/// The generated <c>ICoreBridge</c>, implemented over BoltFFI's"
        )?;
        writeln!(
            w,
            "/// <c>{class}</c>: bytes in, bytes out, nothing else. It is the only"
        )?;
        writeln!(
            w,
            "/// generated type that knows the Rust core exists — pass a fake to"
        )?;
        writeln!(
            w,
            "/// <c>Core(ICoreBridge, IEffectHandler)</c> instead for tests."
        )?;
        writeln!(w, "///")?;
        writeln!(
            w,
            "/// Disposing an <c>FfiBridge</c> releases the core it wraps."
        )?;
        writeln!(w, "/// </summary>")?;
        writeln!(
            w,
            "public sealed class FfiBridge : ICoreBridge, IDisposable"
        )?;
        writeln!(w, "{{")?;
        w.indent();
        writeln!(w, "private readonly {class} _ffi = new();")?;
        writeln!(w)?;
        writeln!(
            w,
            "public byte[] Update(byte[] {event}) => _ffi.Update({event});"
        )?;
        writeln!(w)?;
        writeln!(
            w,
            "public byte[] Resolve(uint id, byte[] output) => _ffi.Resolve(id, output);"
        )?;
        writeln!(w)?;
        writeln!(w, "public byte[] View() => _ffi.View();")?;
        writeln!(w)?;
        writeln!(w, "public void Dispose() => _ffi.Dispose();")?;
        w.unindent();
        writeln!(w, "}}")
    })
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
    ffi: Option<&BoltFfi>,
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
    let view_model_bincode = if config.is_unit_enum(&app.view_model) {
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
        "/// implement it. <c>PropertyChanged</c> may be raised on a thread-pool"
    )?;
    writeln!(
        w,
        "/// thread after an asynchronous request, so marshal to your UI thread in"
    )?;
    writeln!(w, "/// the handler.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public sealed class Core : INotifyPropertyChanged")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "private readonly ICoreBridge _bridge;")?;
    writeln!(w, "private readonly EffectDispatcher _dispatcher;")?;
    writeln!(w, "private {view_model} _view;")?;
    writeln!(w)?;
    writeln!(
        w,
        "public event PropertyChangedEventHandler? PropertyChanged;"
    )?;
    emit_view_property(w, render.name, &view_model)?;
    writeln!(w)?;
    writeln!(w, "public Core(ICoreBridge bridge, IEffectHandler handler)")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_bridge = bridge;")?;
    writeln!(w, "_view = ReadView();")?;
    writeln!(
        w,
        "_dispatcher = new EffectDispatcher(handler, (id, bytes) => Process(_bridge.Resolve(id, bytes)));"
    )?;
    w.unindent();
    writeln!(w, "}}")?;

    if let Some(ffi) = ffi {
        writeln!(w)?;
        writeln!(w, "/// <summary>")?;
        writeln!(
            w,
            "/// Build a <c>Core</c> over the <c>{}</c> the Rust crate exports — the",
            ffi.class_name()
        )?;
        writeln!(w, "/// shell writes the handler and never sees the bridge.")?;
        writeln!(w, "/// </summary>")?;
        writeln!(
            w,
            "public Core(IEffectHandler handler) : this(new FfiBridge(), handler) {{}}"
        )?;
    }

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

/// `View` is a property rather than an auto-property so that replacing it
/// raises `PropertyChanged`, which is how a shell learns the core rendered.
fn emit_view_property(w: &mut dyn IndentWrite, render: &str, view_model: &str) -> io::Result<()> {
    writeln!(w)?;
    writeln!(w, "/// <summary>")?;
    writeln!(w, "/// The view model as of the last <c>{render}</c>.")?;
    writeln!(w, "/// </summary>")?;
    writeln!(w, "public {view_model} View")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "get => _view;")?;
    writeln!(w, "private set")?;
    writeln!(w, "{{")?;
    w.indent();
    writeln!(w, "_view = value;")?;
    writeln!(
        w,
        "PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(View)));"
    )?;
    w.unindent();
    writeln!(w, "}}")?;
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
