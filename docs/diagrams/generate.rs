#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
edition = "2024"
---
//! Generate the Crux book diagrams as hand-written SVG.
//!
//! This script writes these files to docs/src:
//!
//! - command_overview.svg, the round trip between the shell and the core
//! - architecture.svg, the same diagram for the README
//! - crux.svg, the driving and driven sides of the shell around the core
//! - shell_apple.svg, shell_android.svg and shell_react.svg, the codegen and
//!   BoltFFI steps from the shared crate to each app
//! - shell_leptos.svg, the steps from the shared crate to the Leptos web app
//! - part-4/effect_life_cycle.svg, the round trip of one effect through the
//!   executor, the task, the effects channel, the core and the shell
//! - part-4/request_nesting.svg, an operation inside a `Request` inside an
//!   `Effect` variant
//! - part-4/generated_core.svg, the layers of a shell that uses the generated
//!   `Core`, from the UI down to the Rust core
//! - part-2/mocking_layers.svg, the layers of the app at run time and in a
//!   test that uses fakes
//! - part-2/app_lifecycle.svg, the phases of the weather app's `Model` and
//!   the transitions between them
//! - part-2/nested_state_machines.svg, the events that go down the weather
//!   app's model tree and the transitions that come back up
//! - part-3/effect_router.svg, the core and the routing closure in an
//!   `EffectRouter`, and the routes that send effects to a Rust handler, the
//!   shell and a custom FFI
//! - part-3/effect_router_counter.svg, the same diagram for the
//!   counter-routing example
//!
//! Do not edit the SVG files directly. Change this script and run it again.
//! The script needs a nightly Rust toolchain, because cargo runs single-file
//! scripts only on nightly (`-Zscript`):
//!
//!     just diagrams        (from docs/)
//!     ./diagrams/generate.rs [OUT_DIR]
//!
//! The SVG files use system fonts and a transparent background, so that they show
//! on light and dark themes in the book and on GitHub.
//!
//! Every number in the SVG goes through `n`, which rounds to one decimal place
//! and removes trailing zeros (`100`, `12.5`, `33.3`).

use std::fmt::{self, Display, Write};

const LINE: &str = "#8a8a8a";
const INK: &str = "#1a1a1a";
const SANS: &str = r#"system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif"#;
const MONO: &str = r#"ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace"#;

// Attributes of the common text styles.
const HEADING: &str = r#"class="h""#;
const SMALL: &str = r#"class="s""#;
const TITLE: &str = r#"class="b""#;
const CODE: &str = r#"class="b m""#;

/// Append one line to a `String`. Writing to a `String` cannot fail.
macro_rules! emit {
    ($out:expr, $($arg:tt)*) => {
        writeln!($out, $($arg)*).unwrap()
    };
}

/// A number in an SVG attribute: one decimal place, without trailing zeros.
struct Num(f64);

fn n(x: f64) -> Num {
    Num(x)
}

impl Display for Num {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = format!("{:.1}", self.0);
        f.write_str(s.trim_end_matches('0').trim_end_matches('.'))
    }
}

/// An SVG path, built from commands that take absolute coordinates.
struct Path(String);

impl Path {
    fn new() -> Self {
        Path(String::new())
    }

    fn command(mut self, name: char, args: &[f64]) -> Self {
        if !self.0.is_empty() {
            self.0.push(' ');
        }
        self.0.push(name);
        for (i, &arg) in args.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            write!(self.0, "{}", n(arg)).unwrap();
        }
        self
    }

    fn m(self, x: f64, y: f64) -> Self {
        self.command('M', &[x, y])
    }

    fn h(self, x: f64) -> Self {
        self.command('H', &[x])
    }

    fn v(self, y: f64) -> Self {
        self.command('V', &[y])
    }

    fn q(self, x1: f64, y1: f64, x: f64, y: f64) -> Self {
        self.command('Q', &[x1, y1, x, y])
    }

    fn c(self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Self {
        self.command('C', &[x1, y1, x2, y2, x, y])
    }

    fn z(self) -> Self {
        self.command('Z', &[])
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A kind of box, which sets its colours.
#[derive(Clone, Copy)]
enum Kind {
    Shell,
    Fn,
    Model,
    Cmd,
    Effect,
    Event,
    World,
}

impl Kind {
    /// The (fill, stroke) colours.
    fn colours(self) -> (&'static str, &'static str) {
        match self {
            Kind::Shell | Kind::Event => ("#bfdbfe", "#3b82f6"),
            Kind::Fn => ("#bbf7d0", "#22c55e"),
            Kind::Model => ("#fde68a", "#d97706"),
            Kind::Cmd => ("#ddd6fe", "#8b5cf6"),
            Kind::Effect => ("#fecaca", "#ef4444"),
            Kind::World => ("#e5e7eb", "#9ca3af"),
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl Rect {
    fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Rect { x, y, w, h }
    }

    fn cx(&self) -> f64 {
        self.x + self.w / 2.0
    }

    fn cy(&self) -> f64 {
        self.y + self.h / 2.0
    }
}

/// Escape only &, < and >.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn head(out: &mut String, w: f64, h: f64, title: &str, desc: &str) {
    let (w, h) = (n(w), n(h));
    let (title, desc) = (escape(title), escape(desc));
    emit!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-labelledby="title desc">
<title id="title">{title}</title>
<desc id="desc">{desc}</desc>
<style>
text {{ font-family: {SANS}; fill: {INK}; }}
.m {{ font-family: {MONO}; font-variant-ligatures: none; }}
.h {{ font-size: 18px; font-weight: 700; fill: {LINE}; }}
.b {{ font-size: 16px; font-weight: 600; }}
.s {{ font-size: 12px; }}
.l {{ font-size: 12.5px; }}
.ln {{ stroke: {LINE}; stroke-width: 2; fill: none; }}
.dash {{ stroke-dasharray: 6 5; }}
.pill {{ fill: #f2f2f2; stroke: #c4c4c4; stroke-width: 1; }}
@media (prefers-color-scheme: dark) {{
  .h {{ fill: #a8a8a8; }}
}}
</style>
<defs>
<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
<path d="M0,0 L10,5 L0,10 z" fill="{LINE}"/>
</marker>
</defs>"#
    );
}

fn text(out: &mut String, x: f64, y: f64, anchor: &str, attrs: &str, content: &str) {
    let (x, y, content) = (n(x), n(y), escape(content));
    emit!(
        out,
        r#"<text x="{x}" y="{y}" text-anchor="{anchor}" {attrs}>{content}</text>"#
    );
}

/// A tinted area that groups boxes.
fn container(out: &mut String, r: Rect, kind: Kind) {
    let (_, stroke) = kind.colours();
    let (x, y, w, h) = (n(r.x), n(r.y), n(r.w), n(r.h));
    emit!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="14" fill="{stroke}" fill-opacity="0.08" stroke="{stroke}" stroke-width="1.5"/>"#
    );
}

/// A rounded box. A dashed outline marks an optional part.
fn card(out: &mut String, r: Rect, kind: Kind, dashed: bool) {
    let (fill, stroke) = kind.colours();
    let dash = if dashed {
        r#" stroke-dasharray="5 4""#
    } else {
        ""
    };
    let (x, y, w, h) = (n(r.x), n(r.y), n(r.w), n(r.h));
    emit!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="{stroke}" stroke-width="1.5"{dash}/>"#
    );
}

/// A dashed grey card with a darker, heavier outline than `card`, so that the
/// dashes show on a dark page as well as a white one.
fn bold_dashed_card(out: &mut String, r: Rect) {
    let (fill, _) = Kind::World.colours();
    let (x, y, w, h) = (n(r.x), n(r.y), n(r.w), n(r.h));
    emit!(
        out,
        r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="#6b7280" stroke-width="2" stroke-dasharray="8 5"/>"##
    );
}

/// A box with a title and an optional subtitle.
fn boxed(out: &mut String, r: Rect, kind: Kind, attrs: &str, title: &str, sub: Option<&str>) {
    card(out, r, kind, false);
    match sub {
        Some(sub) => {
            text(out, r.cx(), r.cy() - 3.0, "middle", attrs, title);
            text(out, r.cx(), r.cy() + 15.0, "middle", SMALL, sub);
        }
        None => text(out, r.cx(), r.cy() + 5.5, "middle", attrs, title),
    }
}

/// A label on a pill.
fn pill(out: &mut String, cx: f64, cy: f64, label: &str, mono: bool, size: f64) {
    // generous width estimate: 0.62em per char
    let w = label.chars().count() as f64 * size * 0.62 + 16.0;
    let h = size + 10.0;
    let (x, y, rx) = (n(cx - w / 2.0), n(cy - h / 2.0), n(h / 2.0));
    let (w, h) = (n(w), n(h));
    emit!(
        out,
        r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" class="pill"/>"#
    );
    let class = if mono {
        r#"class="l m""#
    } else {
        r#"class="l""#
    };
    text(out, cx, cy + size * 0.36, "middle", class, label);
}

#[derive(Clone, Copy)]
enum Arrow {
    Solid,
    Dashed,
    Both,
}

fn arrow(out: &mut String, path: Path, kind: Arrow) {
    let class = if matches!(kind, Arrow::Dashed) {
        "ln dash"
    } else {
        "ln"
    };
    let start = if matches!(kind, Arrow::Both) {
        r#" marker-start="url(#a)""#
    } else {
        ""
    };
    emit!(
        out,
        r#"<path d="{path}" class="{class}" marker-end="url(#a)"{start}/>"#
    );
}

/// A horizontal arrow from `x0` to `x1` at height `y`.
fn h_arrow(out: &mut String, x0: f64, y: f64, x1: f64) {
    arrow(out, Path::new().m(x0, y).h(x1), Arrow::Solid);
}

/// A thin dashed line without an arrowhead.
fn thin_dash(out: &mut String, path: Path) {
    emit!(
        out,
        r#"<path d="{path}" class="ln dash" style="stroke-width:1.5"/>"#
    );
}

// ---------------------------------------------------------------------------
// Shell + Core round trip (command_overview.svg and architecture.svg)
// ---------------------------------------------------------------------------

/// A box with two offset cards behind it, the notation for 'many'.
fn stack(out: &mut String, r: Rect, kind: Kind, title: &str, sub: &str, dashed: bool) {
    for offset in [12.0, 6.0] {
        let behind = Rect::new(r.x + offset, r.y + offset, r.w, r.h);
        card(out, behind, kind, dashed);
    }
    if dashed {
        card(out, r, kind, true);
        text(out, r.cx(), r.cy() - 3.0, "middle", TITLE, title);
        text(out, r.cx(), r.cy() + 14.0, "middle", SMALL, sub);
    } else {
        boxed(out, r, kind, TITLE, title, Some(sub));
    }
}

/// UML-style multiplicity tag.
fn many(out: &mut String, x: f64, y: f64, anchor: &str) {
    let attrs = format!(r#"class="s m" style="fill:{LINE};font-weight:600""#);
    text(out, x, y, anchor, &attrs, "0..n");
}

fn round_trip(detail: bool) -> String {
    let pick = |a: f64, b: f64| if detail { a } else { b };
    let wide = pick(110.0, 0.0); // extra width for the wide resolve label
    let gap = pick(60.0, 0.0); // extra gap between the shell and the core
    let rust = pick(30.0, 0.0); // extra room for the Rust handlers' label
    let w = 960.0 + wide + gap + rust;
    let (sx, sw) = (30.0, 230.0); // shell
    let bx = 370.0 + (gap / 2.0).floor(); // boundary
    let (cx, cw) = (480.0 + gap, 450.0 + wide + rust); // core
    let top = 60.0;
    let lx = cx + 30.0; // core left column
    let lw = 200.0;
    let (mx, mw) = (810.0 + gap, 100.0); // model column
    let (r1, r2, r3) = (80.0, 200.0, 340.0); // row tops
    let rh = 70.0;
    let (ux, uw) = (sx + 20.0, sw - 50.0); // shell boxes (room for the stack offset)
    let (hy, hh) = (pick(465.0, 425.0), 64.0); // effect handlers (front card)
    let bot = hy + hh + 12.0 + 14.0;
    let h = bot + pick(25.0, 30.0);

    let (title, desc) = if detail {
        (
            "Crux: the shell and core round trip",
            "The shell calls process_event with an event. App::update changes the \
             Model and returns a Command. The Command can send follow-up events back \
             to update, and it gives zero or more effects to the shell, and each effect carries a request. A Render effect \
             goes to the UI, which calls view and draws the ViewModel. The \
             Command sends the other effects to the shell's effect handlers, and the shell \
             resolves each request with the operation's output. Optionally, the Command sends \
             some effects to Rust handlers in the core, which resolve each request the same way.",
        )
    } else {
        (
            "Crux architecture",
            "The shell holds the UI and the effect handlers. The core holds the app's \
             update and view functions, the Model and the Commands. The shell sends \
             events to the core. The core gives the shell zero or more effects. The UI \
             asks the core for the view when it receives a Render effect, and it draws the ViewModel. The effect \
             handlers run the other effects and return each Output to the core.",
        )
    };

    let mut s = String::new();
    head(&mut s, w, h, title, desc);

    if detail {
        text(&mut s, sx + sw / 2.0, 40.0, "middle", HEADING, "Shell");
        text(&mut s, cx + cw / 2.0, 40.0, "middle", HEADING, "Core");
        let faint = format!(r#"class="s" style="fill:{LINE};font-weight:600""#);
        text(&mut s, bx, 26.0, "middle", &faint, "FFI + generated types");
        text(&mut s, bx, 42.0, "middle", &faint, "(for non-Rust shells)");
        container(
            &mut s,
            Rect::new(sx, top - 5.0, sw, bot - top + 10.0),
            Kind::Shell,
        );
        container(
            &mut s,
            Rect::new(cx, top - 5.0, cw, bot - top + 10.0),
            Kind::Fn,
        );
        thin_dash(&mut s, Path::new().m(bx, top - 8.0).v(bot + 10.0));
    } else {
        // Core nested inside the Shell
        container(
            &mut s,
            Rect::new(15.0, 15.0, w - 30.0, h - 30.0),
            Kind::Shell,
        );
        text(&mut s, 35.0, 44.0, "start", HEADING, "Shell");
        container(&mut s, Rect::new(cx, 28.0, cw, bot - 23.0), Kind::Fn);
        text(&mut s, cx + 18.0, 57.0, "start", HEADING, "Core");
    }

    // UI box: text sits left of centre, to leave room for the Render -> view path
    let render_x = ux + uw / 2.0 + 22.0; // x of the Render path inside the UI
    card(
        &mut s,
        Rect::new(ux, r1, uw, r2 + rh - r1),
        Kind::Shell,
        false,
    );
    let tx = (ux + render_x) / 2.0 - 8.0;
    let ty = (r1 + r2 + rh) / 2.0;
    text(&mut s, tx, ty - 10.0, "middle", TITLE, "UI");
    text(&mut s, tx, ty + 10.0, "middle", SMALL, "sends events,");
    text(&mut s, tx, ty + 26.0, "middle", SMALL, "draws the view");
    // Render arrives at the bottom of the UI, and the view request leaves at the top
    thin_dash(
        &mut s,
        Path::new()
            .m(render_x, r2 + rh - 4.0)
            .v(r1 + 40.0)
            .q(render_x, r1 + 22.0, render_x + 18.0, r1 + 22.0)
            .h(ux + uw),
    );
    let handlers = Rect::new(ux, hy, uw, hh);
    stack(
        &mut s,
        handlers,
        Kind::Shell,
        "Effect handlers",
        "HTTP, storage, time, \u{2026}",
        false,
    );

    // core boxes
    boxed(
        &mut s,
        Rect::new(lx, r1, lw, rh),
        Kind::Fn,
        CODE,
        "App::view",
        None,
    );
    boxed(
        &mut s,
        Rect::new(lx, r2, lw, rh),
        Kind::Fn,
        CODE,
        "App::update",
        None,
    );
    let small_code = r#"class="b m" style="font-size:13.5px""#;
    let command = Rect::new(lx, r3, lw, rh);
    boxed(
        &mut s,
        command,
        Kind::Cmd,
        small_code,
        "Command<Effect, Event>",
        None,
    );
    boxed(
        &mut s,
        Rect::new(mx, r1, mw, r2 + rh - r1),
        Kind::Model,
        TITLE,
        "Model",
        None,
    );

    // core internal arrows
    let mid_model = (lx + lw + mx) / 2.0;
    h_arrow(&mut s, mx, r1 + 35.0, lx + lw + 2.0);
    pill(&mut s, mid_model, r1 + 20.0, "reads", false, 12.0);
    arrow(
        &mut s,
        Path::new().m(lx + lw + 2.0, r2 + 35.0).h(mx - 2.0),
        Arrow::Both,
    );
    pill(&mut s, mid_model, r2 + 20.0, "changes", false, 12.0);
    // update -> Command (returns) and follow-up Event loop
    let gy = (r2 + rh + r3) / 2.0;
    arrow(
        &mut s,
        Path::new().m(lx + 60.0, r2 + rh + 2.0).v(r3 - 2.0),
        Arrow::Solid,
    );
    pill(&mut s, lx + 16.0, gy, "returns", false, 12.0);
    let xu = lx + lw - 60.0;
    arrow(
        &mut s,
        Path::new().m(xu, r3 - 2.0).v(r2 + rh + 2.0),
        Arrow::Dashed,
    );
    many(&mut s, xu - 12.0, r2 + rh + 20.0, "end");
    pill(&mut s, xu + 76.0, gy + 8.0, "follow-up Event", false, 12.0);

    // boundary arrows
    let (from, to) = (ux + uw + 2.0, lx - 2.0);
    let mid = (from + to) / 2.0;
    let (t_view, t_vm, t_ev, t_eff, t_res) = if detail {
        (
            "view() on Render",
            "ViewModel",
            "process_event(event)",
            "Effect(Request<Op>)",
            "resolve(request, Op::Output)",
        )
    } else {
        ("on Render", "ViewModel", "Event", "Effect", "Output")
    };
    h_arrow(&mut s, from, r1 + 22.0, to);
    pill(&mut s, mid, r1 + 8.0, t_view, detail, 12.5);
    h_arrow(&mut s, to, r1 + 50.0, from);
    pill(&mut s, mid, r1 + 64.0, t_vm, true, 12.5);
    h_arrow(&mut s, from, r2 + 35.0, to);
    pill(&mut s, mid, r2 + 21.0, t_ev, true, 12.5);

    let jx = ux + uw / 2.0;
    let ey = r3 + 22.0;
    let cmd_bottom = r3 + rh + 2.0;
    let handlers_top = hy - 2.0;
    if detail {
        // Render goes from the Command straight to the UI.
        arrow(
            &mut s,
            Path::new()
                .m(to, ey)
                .h(render_x + 45.0)
                .c(render_x, ey, render_x, ey, render_x, ey - 45.0)
                .v(r2 + rh + 2.0),
            Arrow::Solid,
        );
        pill(&mut s, mid, ey - 14.0, "Render", true, 12.5);
        // All handler arrows meet the bottom of the Command. From left to
        // right: shell effects out, shell result in, Rust result in, Rust
        // effects out. The two pairs mirror each other, so no lines cross.
        let ly = r3 + rh + 30.0;
        let oy = hy + hh - 4.0;
        let (xs, ox) = (lx + 20.0, lx + 70.0); // shell: effects out, result in
        let (xr, xo) = (lx + lw - 80.0, lx + lw - 20.0); // Rust: result in, effects out
        arrow(
            &mut s,
            Path::new()
                .m(xs, cmd_bottom)
                .v(ly - 20.0)
                .c(xs, ly, xs, ly, xs - 20.0, ly)
                .h(jx + 30.0)
                .c(jx, ly, jx, ly, jx, ly + 22.0)
                .v(handlers_top),
            Arrow::Solid,
        );
        many(&mut s, xs - 12.0, ly + 18.0, "end");
        pill(&mut s, mid, ly - 14.0, t_eff, true, 12.5);
        arrow(
            &mut s,
            Path::new()
                .m(ux + uw + 14.0, oy)
                .h(ox - 40.0)
                .c(ox, oy, ox, oy, ox, oy - 30.0)
                .v(cmd_bottom),
            Arrow::Solid,
        );
        pill(&mut s, mid, oy - 14.0, t_res, true, 12.5);

        // Optional Rust handlers in the core, level with the shell's handlers.
        // The dashed outline marks them as optional.
        let rw = 130.0;
        let rx = cx + cw - 12.0 - 8.0 - rw;
        let rin = rx + 40.0; // where the effects go into the stack
        arrow(
            &mut s,
            Path::new()
                .m(xo, cmd_bottom)
                .v(ly - 20.0)
                .c(xo, ly, xo, ly, xo + 20.0, ly)
                .h(rin - 30.0)
                .c(rin, ly, rin, ly, rin, ly + 22.0)
                .v(handlers_top),
            Arrow::Solid,
        );
        many(&mut s, xo + 12.0, ly + 18.0, "start"); // right of the line, below its turn
        pill(
            &mut s,
            (xo + 20.0 + rin - 30.0) / 2.0,
            ly - 14.0,
            t_eff,
            true,
            12.5,
        );
        let rust_handlers = Rect::new(rx, hy, rw, hh);
        stack(
            &mut s,
            rust_handlers,
            Kind::Fn,
            "Rust handlers",
            "optional",
            true,
        );
        arrow(
            &mut s,
            Path::new()
                .m(rx - 2.0, oy)
                .h(xr + 40.0)
                .c(xr, oy, xr, oy, xr, oy - 30.0)
                .v(cmd_bottom),
            Arrow::Solid,
        );
        pill(&mut s, (xr + 40.0 + rx) / 2.0, oy - 14.0, t_res, true, 12.5);
    } else {
        // effects: Command -> shell, then fan out to the UI (Render) and the handlers
        let fork = jx + 100.0;
        emit!(
            s,
            r#"<path d="{}" class="ln"/>"#,
            Path::new().m(to, ey).h(fork)
        );
        many(&mut s, cx - 8.0, ey + 18.0, "end");
        pill(&mut s, mid, ey - 14.0, t_eff, true, 12.5);
        arrow(
            &mut s,
            Path::new()
                .m(fork, ey)
                .c(render_x, ey, render_x, ey, render_x, ey - 45.0)
                .v(r2 + rh + 2.0),
            Arrow::Solid,
        );
        pill(
            &mut s,
            render_x - 48.0,
            r2 + rh + 28.0,
            "Render",
            true,
            12.5,
        );
        arrow(
            &mut s,
            Path::new()
                .m(fork, ey)
                .c(jx, ey, jx, ey, jx, ey + 50.0)
                .v(handlers_top),
            Arrow::Solid,
        );
        // handlers -> Command (Output / resolve)
        let (oy, ox) = (hy + hh / 2.0 + 4.0, lx + 40.0);
        arrow(
            &mut s,
            Path::new()
                .m(ux + uw + 14.0, oy)
                .h(ox - 60.0)
                .c(ox, oy, ox, oy, ox, oy - 40.0)
                .v(r3 + rh + 2.0),
            Arrow::Solid,
        );
        pill(&mut s, mid, oy - 14.0, t_res, true, 12.5);
    }

    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// Ports and adapters (crux.svg)
// ---------------------------------------------------------------------------

const CX: f64 = 480.0; // centre of the rings
const CY: f64 = 318.0;
const RO: f64 = 258.0; // outer radius of the shell ring
const RC: f64 = 140.0; // radius of the core circle
const RM: f64 = 215.0; // radius of the ring labels, outside the tags
const FS: f64 = 13.5; // font size of the ring labels
const LH: f64 = FS * 1.15; // line height of the ring labels

/// A box as [left, top, right, bottom].
type Bounds = [f64; 4];

fn disjoint(a: &Bounds, b: &Bounds) -> bool {
    a[2] < b[0] || a[0] > b[2] || a[3] < b[1] || a[1] > b[3]
}

/// The point at radius `r` and screen angle `deg` (0 = right, 90 = down).
fn polar(r: f64, deg: f64) -> (f64, f64) {
    let t = deg.to_radians();
    (CX + r * t.cos(), CY + r * t.sin())
}

/// Whether a box stays inside the ring band, with a margin of 4px.
fn in_band(b: &Bounds) -> bool {
    let [x0, y0, x1, y1] = *b;
    // nearest and farthest points of the box from the centre
    let near = (CX.clamp(x0, x1) - CX).hypot(CY.clamp(y0, y1) - CY);
    let far = [(x0, y0), (x0, y1), (x1, y0), (x1, y1)]
        .iter()
        .map(|(x, y)| (x - CX).hypot(y - CY))
        .fold(f64::MIN, f64::max);
    near >= RC + 4.0 && far <= RO - 4.0
}

/// Ring labels: horizontal text, centred on the middle of the ring band.
/// Each label is checked so that its box stays inside the band and does not
/// overlap another label. Each label sits on a pill, as the arrow labels in
/// the other diagrams do. The checks use the pill box, not the text box.
fn ring_labels(out: &mut String, items: &[(&str, f64)], placed: &mut Vec<Bounds>) {
    // Try small moves in radius and angle, nearest first.
    let mut moves: Vec<(i32, i32)> = (-20..=10)
        .step_by(5)
        .flat_map(|dr| (-8..=8).step_by(2).map(move |da| (dr, da)))
        .collect();
    moves.sort_by_key(|&(dr, da)| (da.abs(), dr.abs()));

    for &(label, deg) in items {
        let lines: Vec<&str> = label.lines().collect();
        let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
        let w = longest as f64 * FS * 0.6 + 16.0;
        let h = LH * lines.len() as f64 + 8.0;
        let (x, y, b) = moves
            .iter()
            .find_map(|&(dr, da)| {
                let (x, y) = polar(RM + f64::from(dr), deg + f64::from(da));
                let b = [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0];
                (in_band(&b) && placed.iter().all(|o| disjoint(&b, o))).then_some((x, y, b))
            })
            .unwrap_or_else(|| panic!("no place for {label:?} at {deg}"));
        placed.push(b);
        emit!(
            out,
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" class="pill"/>"#,
            n(b[0]),
            n(b[1]),
            n(w),
            n(h),
            n((h / 2.0).min(11.0))
        );
        let first = y - h / 2.0 + 4.0 + LH * 0.78;
        let attrs = format!(r#"style="font-size:{}px;font-weight:600""#, n(FS));
        for (i, line) in lines.iter().enumerate() {
            text(out, x, first + i as f64 * LH, "middle", &attrs, line);
        }
    }
}

/// A tag that sits across the core boundary, because events and effects cross
/// it. A small arrowhead on the leading edge shows the direction: an event
/// points into the core, and an effect points out to the shell.
struct Tag {
    label: &'static str,
    deg: f64,
    kind: Kind,
    mono: bool,
}

fn tag(out: &mut String, t: &Tag, placed: &mut Vec<Bounds>, core_text: &Bounds) {
    let size = 13.0;
    let w = t.label.chars().count() as f64 * size * 0.62 + 18.0;
    let h = size + 12.0;
    let (mut x, y) = polar(RC, t.deg);
    // +1 points right and -1 points left: towards the centre for an event
    // and away from it for an effect
    let mut dir = t.deg.to_radians().cos().signum();
    if matches!(t.kind, Kind::Event) {
        dir = -dir;
    }
    // centre the group (tag + 3px gap + 10px pointer) on the boundary point
    x -= dir * (3.0 + 10.0) / 2.0;
    let tag_box = [x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0];
    // the arrowhead sits on the side edge
    let tip = x + dir * (w / 2.0 + 3.0);
    let pts = [(tip, y - 6.0), (tip, y + 6.0), (tip + dir * 10.0, y)];
    let xs = pts.map(|p| p.0);
    let ys = pts.map(|p| p.1);
    let min = |v: [f64; 3]| v.into_iter().fold(f64::MAX, f64::min);
    let max = |v: [f64; 3]| v.into_iter().fold(f64::MIN, f64::max);
    let head_box = [min(xs), min(ys), max(xs), max(ys)];
    let pad = 8.0; // keep a visible gap to the ring labels
    let boxes = [tag_box, head_box].map(|b| [b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad]);
    for b in &boxes {
        assert!(
            placed.iter().all(|o| disjoint(b, o)) && disjoint(b, core_text),
            "{} {}",
            t.label,
            t.deg
        );
    }
    placed.extend(boxes);

    let (fill, stroke) = t.kind.colours();
    emit!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="7" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#,
        n(tag_box[0]),
        n(tag_box[1]),
        n(w),
        n(h)
    );
    let class = if t.mono { r#"class="m" "# } else { "" };
    let attrs = format!(r#"{class}style="font-size:{}px;font-weight:600""#, n(size));
    text(out, x, y + size * 0.36, "middle", &attrs, t.label);
    let points: Vec<String> = pts
        .iter()
        .map(|(a, b)| format!("{},{}", n(*a), n(*b)))
        .collect();
    emit!(
        out,
        r#"<polygon points="{}" fill="{stroke}"/>"#,
        points.join(" ")
    );
}

/// The path of a full circle, drawn as two arcs.
fn circle_path(cx: f64, cy: f64, r: f64) -> String {
    let (r, d) = (n(r), n(2.0 * r));
    let (x, y) = (n(cx - r.0), n(cy));
    format!("M{x},{y} a{r},{r} 0 1,0 {d},0 a{r},{r} 0 1,0 -{d},0 Z")
}

fn ports() -> String {
    const W: f64 = 960.0;
    const H: f64 = 665.0;
    let title = "Crux: the two sides of the shell";
    let desc = "The Rust core is in the centre and holds the Model. The shell surrounds the core. Events (blue) cross the \
                boundary into the core. Effects (red) such as Render, Http, KeyValue and Time \
                cross the boundary out to the shell. An arrowhead on each tag shows the \
                direction. On the \
                driving side, the user works with a UI built in SwiftUI, Jetpack Compose, \
                React, Leptos, Yew, Dioxus or Ratatui. On the driven side, the shell runs \
                the effects against APIs, storage and devices on iOS, macOS, Android, the \
                web, the desktop or the terminal.";
    let mut s = String::new();
    head(&mut s, W, H, title, desc);

    // side labels
    text(&mut s, CX - 20.0, 40.0, "end", HEADING, "Driving (UI)");
    text(&mut s, CX + 20.0, 40.0, "start", HEADING, "Driven (I/O)");

    // user
    let ux = 150.0;
    emit!(
        s,
        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" class="ln"/>"#,
        n(ux + 40.0),
        n(CY),
        n(CX - RO),
        n(CY)
    );
    emit!(
        s,
        r#"<circle cx="{}" cy="{}" r="16" fill="none" stroke="{LINE}" stroke-width="2.5"/>"#,
        n(ux),
        n(CY - 38.0)
    );
    let body = Path::new()
        .m(ux - 30.0, CY + 30.0)
        .c(
            ux - 30.0,
            CY - 15.0,
            ux + 30.0,
            CY - 15.0,
            ux + 30.0,
            CY + 30.0,
        )
        .z();
    emit!(
        s,
        r#"<path d="{body}" fill="none" stroke="{LINE}" stroke-width="2.5"/>"#
    );
    text(&mut s, ux, CY + 58.0, "middle", HEADING, "User");

    // external world
    let wx = 830.0;
    for (i, name) in ["APIs", "Storage", "Devices"].into_iter().enumerate() {
        let y = CY - 70.0 + i as f64 * 70.0;
        let link = Path::new()
            .m(CX + RO, CY)
            .c(CX + RO + 50.0, CY, wx - 110.0, y, wx - 60.0, y);
        emit!(s, r#"<path d="{link}" class="ln"/>"#);
        boxed(
            &mut s,
            Rect::new(wx - 60.0, y - 22.0, 120.0, 44.0),
            Kind::World,
            TITLE,
            name,
            None,
        );
    }

    // Shell ring and core circle use the container colours of the other
    // diagrams: a light tint of the kind colour over the page background. The
    // ring is an annulus (even-odd fill), so the core tint sits directly on the
    // page and does not mix with the shell tint. Each outline has the colour
    // of the area that it bounds.
    let (_, shell_tint) = Kind::Shell.colours();
    let (_, core_tint) = Kind::Fn.colours();
    emit!(
        s,
        r#"<path d="{} {}" fill="{shell_tint}" fill-opacity="0.08" fill-rule="evenodd"/>"#,
        circle_path(CX, CY, RO),
        circle_path(CX, CY, RC)
    );
    emit!(
        s,
        r#"<circle cx="{}" cy="{}" r="{}" fill="none" stroke="{shell_tint}" stroke-width="1.5"/>"#,
        n(CX),
        n(CY),
        n(RO)
    );
    emit!(
        s,
        r#"<circle cx="{}" cy="{}" r="{}" fill="{core_tint}" fill-opacity="0.08" stroke="{core_tint}" stroke-width="1.5"/>"#,
        n(CX),
        n(CY),
        n(RC)
    );
    // The divider goes the same short distance out of the ring at the top
    // (between the two side headings) and at the bottom (above the title).
    let ext = 25.0;
    for (y0, y1) in [(CY - RO - ext, CY - RC), (CY + RC, CY + RO + ext)] {
        emit!(
            s,
            r#"<line x1="{x}" y1="{}" x2="{x}" y2="{}" class="ln"/>"#,
            n(y0),
            n(y1),
            x = n(CX)
        );
    }
    let title_y = CY + RO + ext + 42.0;
    text(
        &mut s,
        CX,
        title_y,
        "middle",
        r#"class="h" style="font-size:34px""#,
        "Crux",
    );

    // Rust Core heading and Model box, centred on CY as one group. The group
    // runs from the cap top of the heading to the bottom of the Model box.
    const HEADING_SIZE: f64 = 24.0;
    const MODEL_H: f64 = 30.0;
    let cap = 0.72 * HEADING_SIZE;
    let gap = 12.0; // baseline of the heading to the top of the Model box
    let group_top = CY - (cap + gap + MODEL_H) / 2.0;
    let heading_y = group_top + cap;
    let model_y = heading_y + gap;
    let core_text = [
        CX - 62.0,
        group_top - 5.0,
        CX + 62.0,
        model_y + MODEL_H + 2.0,
    ];

    let mut placed: Vec<Bounds> = Vec::new();
    let tag_at = |label, deg, kind, mono| Tag {
        label,
        deg,
        kind,
        mono,
    };
    let mut tags = String::new();
    for t in [
        tag_at("Event", 212.0, Kind::Event, true),
        tag_at("Render", 148.0, Kind::Effect, true),
        tag_at("Http", -58.0, Kind::Effect, true),
        tag_at("KeyValue", -22.0, Kind::Effect, true),
        tag_at("Time", 14.0, Kind::Effect, true),
        tag_at("Your own", 50.0, Kind::Effect, false),
    ] {
        tag(&mut tags, &t, &mut placed, &core_text);
    }

    ring_labels(
        &mut s,
        &[
            ("SwiftUI", 254.0),
            ("Jetpack\nCompose", 230.0),
            ("React", 198.0),
            ("Leptos", 170.0),
            ("Yew", 146.0),
            ("Dioxus", 125.0),
            ("Ratatui", 103.0),
        ],
        &mut placed,
    );
    ring_labels(
        &mut s,
        &[
            ("iOS", 290.0),
            ("macOS", 314.0),
            ("Android", 336.0),
            ("Web", 12.0),
            ("Desktop", 40.0),
            ("Terminal", 70.0),
        ],
        &mut placed,
    );

    // core contents
    let heading_attrs = format!(r#"class="h" style="font-size:{}px""#, n(HEADING_SIZE));
    text(&mut s, CX, heading_y, "middle", &heading_attrs, "Rust Core");
    boxed(
        &mut s,
        Rect::new(CX - 40.0, model_y, 80.0, MODEL_H),
        Kind::Model,
        TITLE,
        "Model",
        None,
    );
    s.push_str(&tags);

    // Legend: mid-way between the centres of the User label and the Crux
    // title, moved radially away from the ring centre until it is 12px clear
    // of the outer ring.
    let user_c = (ux, CY + 58.0 - 0.72 * 18.0 / 2.0);
    let title_cy = title_y - 0.72 * 34.0 / 2.0;
    let title_left = CX - 4.0 * 34.0 * 0.6 / 2.0;
    let lw = 36.0 + 6.0 * 13.0 * 0.6; // swatch, gap and the longer label
    let lh = 48.0; // rows span ly-11 .. ly+37
    let (mut lcx, mut lcy) = ((user_c.0 + CX) / 2.0, (user_c.1 + title_cy) / 2.0);

    let ring_gap = |mx: f64, my: f64| {
        let (x0, y0, x1, y1) = (mx - lw / 2.0, my - lh / 2.0, mx + lw / 2.0, my + lh / 2.0);
        (CX.clamp(x0, x1) - CX).hypot(CY.clamp(y0, y1) - CY) - RO
    };

    let (dx, dy) = (lcx - CX, lcy - CY);
    let norm = dx.hypot(dy);
    let (dx, dy) = (dx / norm, dy / norm);
    while ring_gap(lcx, lcy) < 12.0 {
        lcx += dx * 0.5;
        lcy += dy * 0.5;
    }
    let lx = lcx - lw / 2.0;
    let ly = lcy - 13.0; // rows span ly-11 .. ly+37, so their centre is ly+13
    let lbox = [lx, ly - 11.0, lx + lw, ly + 37.0];
    assert!(ring_gap(lcx, lcy) >= 12.0, "{}", ring_gap(lcx, lcy));
    assert!(
        lbox[2] < title_left - 12.0 || lbox[1] > title_cy + 20.0,
        "{lbox:?}"
    );
    assert!(lbox[0] > 0.0 && lbox[3] < H, "{lbox:?}");
    assert!(
        lbox[2] < ux - 40.0 || lbox[1] > CY + 58.0 + 8.0 || lbox[0] > ux + 40.0,
        "{lbox:?}"
    );
    for (i, (kind, label)) in [(Kind::Event, "Event"), (Kind::Effect, "Effect")]
        .into_iter()
        .enumerate()
    {
        let (fill, stroke) = kind.colours();
        let y = ly + i as f64 * 30.0;
        emit!(
            s,
            r#"<rect x="{}" y="{}" width="26" height="18" rx="5" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#,
            n(lx),
            n(y - 11.0)
        );
        let attrs = format!(r#"class="s" style="fill:{LINE};font-size:13px;font-weight:600""#);
        text(&mut s, lx + 36.0, y + 3.0, "start", &attrs, label);
    }
    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// Shell build pipelines (shell_*.svg)
// ---------------------------------------------------------------------------

/// The estimated width of a line of text, generous like `pill`: 0.62em per char.
fn text_width(s: &str, size: f64) -> f64 {
    s.chars().count() as f64 * size * 0.62
}

/// One line of text in a `lines_card`: its attributes, font size and content.
type Line<'a> = (&'a str, f64, &'a str);

const LINE_SMALL_CODE: &str = r#"class="s m""#;
const LINE_CMD: &str = r#"class="m" style="font-size:13px;font-weight:600""#;

/// A card with lines of text, centred as one block. The function panics if a
/// line does not fit in the card with 12px of space on each side.
fn lines_card(out: &mut String, r: Rect, kind: Kind, lines: &[Line]) {
    card(out, r, kind, false);
    let height = |size: f64| (size * 1.4).round();
    let total: f64 = lines.iter().map(|&(_, size, _)| height(size)).sum();
    assert!(total + 12.0 <= r.h, "card is too short for {lines:?}");
    let mut top = r.cy() - total / 2.0;
    for &(attrs, size, content) in lines {
        assert!(
            text_width(content, size) + 24.0 <= r.w,
            "{content:?} does not fit in {}px",
            r.w
        );
        let lh = height(size);
        text(
            out,
            r.cx(),
            top + lh / 2.0 + size * 0.36,
            "middle",
            attrs,
            content,
        );
        top += lh;
    }
}

/// A vertical arrow from `y0` down to `y1`, with an optional label on a pill
/// in the middle of the line.
fn down(out: &mut String, x: f64, y0: f64, y1: f64, label: Option<(&str, bool)>) {
    arrow(out, Path::new().m(x, y0 + 2.0).v(y1 - 2.0), Arrow::Solid);
    if let Some((label, mono)) = label {
        pill(out, x, (y0 + y1) / 2.0, label, mono, 12.5);
    }
}

/// The strings for one shell that uses codegen and BoltFFI.
struct Codegen {
    file: &'static str,
    title: &'static str,
    /// The value of `--language` for the codegen binary.
    language: &'static str,
    /// The target of `boltffi pack`.
    pack: &'static str,
    /// The generated types.
    types: &'static str,
    /// The generated bindings, as a title and a subtitle.
    bindings: (&'static str, &'static str),
    /// The lines under `ffi.rs`.
    ffi: &'static [&'static str],
    /// The app, as a title and a subtitle.
    app: (&'static str, &'static str),
    /// The platform, as the description names it.
    platform: &'static str,
}

const CODEGEN: [Codegen; 3] = [
    Codegen {
        file: "shell_apple.svg",
        title: "Crux: build the Apple shell",
        language: "swift",
        pack: "apple",
        types: "Swift types",
        bindings: ("Swift package", "and xcframework"),
        ffi: &["CoreFfi", "#[boltffi::export]"],
        app: ("Apple app", "Swift · SwiftUI"),
        platform: "Apple",
    },
    Codegen {
        file: "shell_android.svg",
        title: "Crux: build the Android shell",
        language: "kotlin",
        pack: "android",
        types: "Kotlin types",
        bindings: ("Kotlin bindings", "and JNI libraries"),
        ffi: &["CoreFfi", "#[boltffi::export]"],
        app: ("Android app", "Kotlin · Jetpack Compose"),
        platform: "Android",
    },
    Codegen {
        file: "shell_react.svg",
        title: "Crux: build the React web shell",
        language: "typescript",
        pack: "wasm",
        types: "TypeScript types",
        bindings: ("WASM package", "and JS bindings"),
        ffi: &["CoreFfi · #[boltffi::export]"],
        app: ("Web app", "TypeScript · React · Next.js"),
        platform: "web",
    },
];

/// The shared crate at the top, two build steps from it, and the app at the
/// bottom that uses the output of both steps.
fn codegen(p: &Codegen) -> String {
    const W: f64 = 640.0;
    const H: f64 = 532.0;
    let (cw, gap) = (270.0, 30.0); // column width and the gap between the columns
    let (c0, c1) = (35.0, 35.0 + cw + gap); // column lefts
    let (m0, m1) = (c0 + cw / 2.0, c1 + cw / 2.0); // column centres
    let (src_y, src_h) = (60.0, 102.0); // source files
    let (cmd_y, cmd_h, cmd_w) = (222.0, 52.0, 230.0); // build steps
    let (gen_y, gen_h) = (344.0, 60.0); // generated files
    let (app_y, app_h) = (450.0, 64.0); // the app

    let (types_title, (bind_title, bind_sub)) = (p.types, p.bindings);
    let desc = format!(
        "The shared Rust crate holds app.rs and ffi.rs. In app.rs, the Event, Effect and \
         ViewModel types derive Facet, and the Effect type has the facet_typegen attribute. \
         In ffi.rs, the CoreFfi type has the boltffi::export attribute. The command \
         cargo run --bin codegen --language {lang} reads app.rs, and typegen makes the \
         {types_title}. The command boltffi pack {pack} reads ffi.rs, and bindgen makes the \
         {bind_title} {bind_sub}. The {platform} app uses the types and the bindings.",
        lang = p.language,
        pack = p.pack,
        platform = p.platform,
    );
    let mut s = String::new();
    head(&mut s, W, H, p.title, &desc);

    container(
        &mut s,
        Rect::new(15.0, 15.0, W - 30.0, src_y + src_h + 18.0 - 15.0),
        Kind::Fn,
    );
    text(&mut s, 35.0, 44.0, "start", HEADING, "shared/ (Rust crate)");
    lines_card(
        &mut s,
        Rect::new(c0, src_y, cw, src_h),
        Kind::Fn,
        &[
            (CODE, 16.0, "app.rs"),
            (LINE_SMALL_CODE, 12.0, "Event · Effect · ViewModel"),
            (LINE_SMALL_CODE, 12.0, "#[derive(Facet)]"),
            (LINE_SMALL_CODE, 12.0, "#[effect(facet_typegen)]"),
        ],
    );
    let mut ffi: Vec<Line> = vec![(CODE, 16.0, "ffi.rs")];
    ffi.extend(p.ffi.iter().map(|&line| (LINE_SMALL_CODE, 12.0, line)));
    lines_card(&mut s, Rect::new(c1, src_y, cw, src_h), Kind::Fn, &ffi);

    // build steps
    let codegen_lang = format!("--language {}", p.language);
    let boltffi = format!("boltffi pack {}", p.pack);
    let steps: [&[Line]; 2] = [
        &[
            (LINE_CMD, 13.0, "cargo run --bin codegen"),
            (LINE_CMD, 13.0, &codegen_lang),
        ],
        &[(LINE_CMD, 13.0, &boltffi)],
    ];
    for (m, lines) in [m0, m1].into_iter().zip(steps) {
        down(&mut s, m, src_y + src_h, cmd_y, None);
        let step = Rect::new(m - cmd_w / 2.0, cmd_y, cmd_w, cmd_h);
        lines_card(&mut s, step, Kind::World, lines);
    }

    // generated files
    down(&mut s, m0, cmd_y + cmd_h, gen_y, Some(("typegen", false)));
    down(&mut s, m1, cmd_y + cmd_h, gen_y, Some(("bindgen", false)));
    lines_card(
        &mut s,
        Rect::new(c0, gen_y, cw, gen_h),
        Kind::Cmd,
        &[(TITLE, 16.0, types_title)],
    );
    lines_card(
        &mut s,
        Rect::new(c1, gen_y, cw, gen_h),
        Kind::Cmd,
        &[(TITLE, 16.0, bind_title), (SMALL, 12.0, bind_sub)],
    );

    // the app
    for m in [m0, m1] {
        down(&mut s, m, gen_y + gen_h, app_y, None);
    }
    lines_card(
        &mut s,
        Rect::new(c0, app_y, c1 + cw - c0, app_h),
        Kind::Shell,
        &[(TITLE, 16.0, p.app.0), (SMALL, 12.0, p.app.1)],
    );
    assert!(app_y + app_h + 18.0 == H);

    s.push_str("</svg>\n");
    s
}

/// The shared crate as a Cargo dependency of the Rust web shell, which trunk
/// builds into the browser-ready app.
fn leptos() -> String {
    const W: f64 = 480.0;
    const H: f64 = 586.0;
    let mid = W / 2.0;
    let (cx, cw) = (45.0, W - 90.0); // cards inside the containers
    let (shared_y, shell_y) = (15.0, 220.0); // container tops
    let (card_h, cont_h) = (60.0, 125.0);
    let (cmd_y, cmd_h, cmd_w) = (cont_h + shell_y + 50.0, 44.0, 200.0);
    let dist_y = cmd_y + cmd_h + 72.0;

    let title = "Crux: build the Leptos web shell";
    let desc = "The shared Rust crate holds app.rs with the App, Event, Effect and \
                ViewModel types. The Rust web shell crate has the shared crate as a \
                dependency in Cargo.toml, so it needs no codegen. The shell uses the \
                shared types natively, with no FFI or serialization. The command trunk \
                build compiles the shell to WASM, and puts the browser-ready app in dist/.";
    let mut s = String::new();
    head(&mut s, W, H, title, desc);

    container(
        &mut s,
        Rect::new(15.0, shared_y, W - 30.0, cont_h),
        Kind::Fn,
    );
    text(
        &mut s,
        35.0,
        shared_y + 29.0,
        "start",
        HEADING,
        "shared/ (Rust crate)",
    );
    lines_card(
        &mut s,
        Rect::new(cx, shared_y + 45.0, cw, card_h),
        Kind::Fn,
        &[
            (CODE, 16.0, "app.rs"),
            (LINE_SMALL_CODE, 12.0, "App · Event · Effect · ViewModel"),
        ],
    );
    down(
        &mut s,
        mid,
        shared_y + cont_h,
        shell_y,
        Some(("Cargo.toml dependency (no codegen)", false)),
    );

    container(
        &mut s,
        Rect::new(15.0, shell_y, W - 30.0, cont_h),
        Kind::Shell,
    );
    text(
        &mut s,
        35.0,
        shell_y + 29.0,
        "start",
        HEADING,
        "Rust web shell crate",
    );
    lines_card(
        &mut s,
        Rect::new(cx, shell_y + 45.0, cw, card_h),
        Kind::Shell,
        &[
            (TITLE, 16.0, "Uses the shared types natively"),
            (SMALL, 12.0, "no FFI or serialization needed"),
        ],
    );

    down(&mut s, mid, shell_y + cont_h, cmd_y, None);
    lines_card(
        &mut s,
        Rect::new(mid - cmd_w / 2.0, cmd_y, cmd_w, cmd_h),
        Kind::World,
        &[(LINE_CMD, 13.0, "trunk build")],
    );
    down(&mut s, mid, cmd_y + cmd_h, dist_y, Some(("WASM", true)));
    lines_card(
        &mut s,
        Rect::new(cx, dist_y, cw, card_h),
        Kind::Cmd,
        &[(CODE, 16.0, "dist/"), (SMALL, 12.0, "browser-ready app")],
    );
    assert!(dist_y + card_h + 15.0 == H, "{}", dist_y + card_h + 15.0);

    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// Command runtime and type generation internals (part-4/*.svg)
// ---------------------------------------------------------------------------

/// The attributes of text in the line colour, which shows on light and dark
/// backgrounds. Use it for text that is not on a card or a pill.
fn faint(size: f64, mono: bool) -> String {
    let class = if mono { "s m" } else { "s" };
    format!(
        r#"class="{class}" style="fill:{LINE};font-size:{}px;font-weight:600""#,
        n(size)
    )
}

/// A legend: a swatch and a label on each row, from the top at `y`.
fn legend(out: &mut String, x: f64, y: f64, rows: &[(Kind, &str)]) {
    for (i, &(kind, label)) in rows.iter().enumerate() {
        let (fill, stroke) = kind.colours();
        let y = y + i as f64 * 30.0;
        emit!(
            out,
            r#"<rect x="{}" y="{}" width="26" height="18" rx="5" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>"#,
            n(x),
            n(y - 11.0)
        );
        text(out, x + 36.0, y + 3.0, "start", &faint(13.0, false), label);
    }
}

/// A participant in a sequence diagram, with a lifeline down from its card.
struct Lane {
    title: &'static str,
    sub: &'static str,
    /// The x of the lifeline.
    x: f64,
    kind: Kind,
}

/// One row of a sequence diagram.
enum Step {
    /// A message from one lane to another, with a label on a pill above the
    /// line. A dashed line is a value that a call returns.
    Msg {
        from: usize,
        to: usize,
        label: &'static str,
        mono: bool,
        dashed: bool,
    },
    /// A note on one lifeline: what that participant does.
    Note {
        lane: usize,
        lines: &'static [&'static str],
    },
}

/// A number in a circle in the left margin, which refers to a numbered step
/// in the text.
fn badge(out: &mut String, x: f64, y: f64, number: u8) {
    emit!(
        out,
        r##"<circle cx="{}" cy="{}" r="11" fill="#f2f2f2" stroke="#c4c4c4" stroke-width="1"/>"##,
        n(x),
        n(y)
    );
    let attrs = r#"class="s" style="font-weight:700""#;
    text(out, x, y + 4.3, "middle", attrs, &number.to_string());
}

/// A sequence diagram: the lanes across the top, and the steps from the top
/// down. A step with a number gets a badge in the left margin.
///
/// The function panics if a label does not fit between the ends of its
/// message, or if a note reaches a lifeline next to its own.
fn sequence(out: &mut String, lanes: &[Lane], steps: &[(Option<u8>, Step)], top: f64) -> f64 {
    const HEAD_H: f64 = 56.0;
    const HEAD_W: f64 = 165.0;
    const MSG_H: f64 = 40.0; // from the top of the pill to the line, and a gap below
    const NOTE_LINE: f64 = 17.0;
    let badge_x = 22.0;

    // First pass: the y of each row, so that the lifelines can go behind.
    let mut ys = Vec::new();
    let mut y = top + HEAD_H + 16.0;
    for (_, step) in steps {
        match step {
            Step::Msg { .. } => {
                y += 30.0;
                ys.push(y);
                y += MSG_H - 30.0;
            }
            Step::Note { lines, .. } => {
                let h = lines.len() as f64 * NOTE_LINE + 12.0;
                ys.push(y + 6.0 + h / 2.0);
                y += h + 12.0;
            }
        }
    }
    let bottom = y + 8.0;

    for lane in lanes {
        thin_dash(out, Path::new().m(lane.x, top + HEAD_H).v(bottom));
        let head = Rect::new(lane.x - HEAD_W / 2.0, top, HEAD_W, HEAD_H);
        let sub_attrs = if lane.sub.contains('_') {
            LINE_SMALL_CODE
        } else {
            SMALL
        };
        lines_card(
            out,
            head,
            lane.kind,
            &[(TITLE, 16.0, lane.title), (sub_attrs, 12.0, lane.sub)],
        );
    }

    for ((number, step), &y) in steps.iter().zip(&ys) {
        if let Some(number) = number {
            badge(out, badge_x, y, *number);
        }
        match *step {
            Step::Msg {
                from,
                to,
                label,
                mono,
                dashed,
            } => {
                let (x0, x1) = (lanes[from].x, lanes[to].x);
                let dir = (x1 - x0).signum();
                let kind = if dashed { Arrow::Dashed } else { Arrow::Solid };
                arrow(out, Path::new().m(x0, y).h(x1 - dir * 2.0), kind);
                let w = text_width(label, 12.5) + 16.0;
                assert!(
                    w + 12.0 <= (x1 - x0).abs(),
                    "{label:?} does not fit between its lanes"
                );
                pill(out, (x0 + x1) / 2.0, y - 14.0, label, mono, 12.5);
            }
            Step::Note { lane, lines } => {
                let x = lanes[lane].x;
                let longest = lines.iter().map(|l| text_width(l, 12.0)).fold(0.0, f64::max);
                let w = longest + 24.0;
                let h = lines.len() as f64 * NOTE_LINE + 12.0;
                for other in lanes {
                    assert!(
                        other.x == x || (other.x - x).abs() > w / 2.0 + 6.0,
                        "{lines:?} reaches the next lifeline"
                    );
                }
                let r = Rect::new(x - w / 2.0, y - h / 2.0, w, h);
                let rows: Vec<Line> = lines.iter().map(|&l| (SMALL, 12.0, l)).collect();
                lines_card(out, r, lanes[lane].kind, &rows);
            }
        }
    }
    bottom
}

/// The life cycle of one effect: from the task that requests it, to the shell
/// that resolves it, and back to the task. The numbers match the steps in
/// part-4/runtime.md.
fn effect_life_cycle() -> String {
    const W: f64 = 980.0;
    let lanes = [
        Lane {
            title: "Executor",
            sub: "run_until_settled",
            x: 135.0,
            kind: Kind::Cmd,
        },
        Lane {
            title: "Task",
            sub: "command future",
            x: 310.0,
            kind: Kind::Cmd,
        },
        Lane {
            title: "Effects",
            sub: "channel",
            x: 485.0,
            kind: Kind::Effect,
        },
        Lane {
            title: "Core",
            sub: "crux_core::Core",
            x: 660.0,
            kind: Kind::Fn,
        },
        Lane {
            title: "Shell",
            sub: "effect handlers",
            x: 880.0,
            kind: Kind::Shell,
        },
    ];
    let (exec, task, chan, core, shell) = (0, 1, 2, 3, 4);
    let msg = |from, to, label, mono| Step::Msg {
        from,
        to,
        label,
        mono,
        dashed: false,
    };
    let ret = |from, to, label, mono| Step::Msg {
        from,
        to,
        label,
        mono,
        dashed: true,
    };
    let note = |lane, lines| Step::Note { lane, lines };
    let steps = [
        (None, msg(shell, core, "process_event(event)", true)),
        (None, note(core, &["update returns a Command"])),
        (Some(1), msg(core, exec, "spawn a task", false)),
        (Some(2), msg(core, exec, "run_until_settled()", true)),
        (None, note(exec, &["TaskId to ready queue"])),
        (Some(3), msg(exec, task, "poll", true)),
        (Some(4), note(task, &["runs to the first .await"])),
        (
            Some(5),
            note(task, &["creates the Request and", "its resolve callback"]),
        ),
        (Some(6), msg(task, chan, "send(Effect)", true)),
        (None, ret(task, exec, "Pending", true)),
        (Some(7), msg(chan, core, "Effect", true)),
        (None, ret(core, shell, "Vec<Effect>", true)),
        (None, note(shell, &["runs the effect"])),
        (Some(8), msg(shell, core, "resolve(request, output)", true)),
        (
            Some(9),
            msg(core, task, "callback sends the output", false),
        ),
        (None, msg(task, exec, "wake", false)),
        (None, note(exec, &["TaskId to ready queue"])),
        (Some(10), msg(core, exec, "run_until_settled()", true)),
        (None, msg(exec, task, "poll", true)),
        (
            Some(11),
            note(task, &["the output is ready, so the", "task continues to the next .await"]),
        ),
        (None, ret(task, exec, "Ready or Pending", false)),
        (None, ret(core, shell, "Vec<Effect>", true)),
    ];

    let title = "Crux: one effect's life cycle";
    let desc = "A sequence diagram with five lanes: the executor, the task, the effects \
                channel, the core and the shell. Solid arrows are calls and messages, and \
                dashed arrows are return values. The shell calls process_event, and update \
                returns a Command. The core spawns the Command's task and calls \
                run_until_settled, which puts the task on the ready queue and polls it. The \
                task runs to its first .await, where it creates a Request with a resolve \
                callback. The task sends the Effect on the effects channel and returns \
                Pending. The core takes the Effect from the channel and returns it to the \
                shell. The shell runs the effect and calls resolve with the output. The \
                resolve callback sends the output to the task, and the waker puts the task \
                on the ready queue again. The core calls run_until_settled, the executor \
                polls the task, and the task continues to its next .await or to its end. \
                The core returns any new effects to the shell.";

    let mut body = String::new();
    let bottom = sequence(&mut body, &lanes, &steps, 20.0);
    let mut s = String::new();
    head(&mut s, W, bottom + 15.0, title, desc);
    s.push_str(&body);
    s.push_str("</svg>\n");
    s
}

/// A tinted area with a heading in its top-left corner.
fn group(out: &mut String, r: Rect, kind: Kind, heading: &str, mono: bool) {
    container(out, r, kind);
    text(
        out,
        r.x + 18.0,
        r.y + 27.0,
        "start",
        &faint(15.0, mono),
        heading,
    );
}

/// How an operation is wrapped on its way to the shell: the operation and its
/// resolve handle make a `Request`, and a variant of the `Effect` enum holds
/// the `Request`.
fn request_nesting() -> String {
    const W: f64 = 700.0;
    const H: f64 = 290.0;
    let title = "Crux: an effect wraps a request, which wraps an operation";
    let desc = "Three nested boxes. The outer box is the Effect variant \
                Effect::Http, which holds a Request<HttpRequest>. The Request holds two \
                fields. The operation field holds the HttpRequest, which implements \
                Operation with an Output type of HttpResult. The handle field holds a \
                RequestHandle<HttpResult>, which is Never for a notification, Once for a \
                request and Many for a stream.";
    let mut s = String::new();
    head(&mut s, W, H, title, desc);

    group(
        &mut s,
        Rect::new(15.0, 15.0, W - 30.0, H - 30.0),
        Kind::Effect,
        "Effect::Http(Request<HttpRequest>)",
        true,
    );
    text(
        &mut s,
        W - 33.0,
        42.0,
        "end",
        &faint(12.0, false),
        "one variant per operation",
    );
    let req = Rect::new(35.0, 62.0, W - 70.0, H - 62.0 - 35.0);
    group(&mut s, req, Kind::Shell, "Request<HttpRequest>", true);

    let (cy, ch) = (req.y + 46.0, 130.0); // the field cards
    let op = Rect::new(req.x + 20.0, cy, 250.0, ch);
    lines_card(
        &mut s,
        op,
        Kind::Model,
        &[
            (CODE, 16.0, "operation"),
            (LINE_SMALL_CODE, 12.0, "HttpRequest"),
            (SMALL, 12.0, "implements Operation"),
            (LINE_SMALL_CODE, 12.0, "type Output = HttpResult"),
        ],
    );
    let hd = Rect::new(op.x + op.w + 20.0, cy, req.x + req.w - 20.0 - (op.x + op.w + 20.0), ch);
    card(&mut s, hd, Kind::World, false);
    text(&mut s, hd.cx(), hd.y + 28.0, "middle", CODE, "handle");
    text(
        &mut s,
        hd.cx(),
        hd.y + 48.0,
        "middle",
        LINE_SMALL_CODE,
        "RequestHandle<HttpResult>",
    );
    for (i, (variant, kind)) in [("Never", "notify"), ("Once", "request"), ("Many", "stream")]
        .into_iter()
        .enumerate()
    {
        let x = hd.cx() + (i as f64 - 1.0) * 92.0;
        pill(&mut s, x, hd.y + 80.0, variant, true, 12.5);
        text(&mut s, x, hd.y + 112.0, "middle", SMALL, kind);
    }
    assert!(req.y + req.h - (cy + ch) >= 15.0);

    s.push_str("</svg>\n");
    s
}

/// The layers of a shell that uses the generated `Core`, from the UI down to
/// the Rust core. Purple boxes are generated and blue boxes are the shell's
/// own code.
fn generated_core() -> String {
    const W: f64 = 950.0;
    const H: f64 = 675.0;
    let (ax, aw) = (40.0, 210.0); // column A: UI, Core
    let (bx, bw) = (360.0, 210.0); // column B: EffectDispatcher
    let (cx, cw) = (710.0, 200.0); // column C: EffectHandler, legend
    let (r1, r1h) = (64.0, 60.0); // UI
    let (r2, r2h) = (184.0, 70.0); // Core, EffectDispatcher, EffectHandler
    let (r3, r3h) = (314.0, 60.0); // FfiBridge
    let (r4, r4h) = (424.0, 60.0); // CoreFfi bindings
    let shell_bottom = r4 + r4h + 20.0;
    let rust_top = shell_bottom + 30.0;
    let (r5, r5h) = (rust_top + 46.0, 60.0); // the Rust core
    let wide = bx + bw - ax; // the bridge rows span columns A and B

    let title = "Crux: the generated Core";
    let desc = "The layers of a shell that uses the generated Core, from the UI at the top \
                to the Rust core at the bottom. Purple boxes are generated, blue boxes are \
                the shell's own code, and green boxes are the Rust shared crate. The UI \
                reads the view from Core and calls update with an event. Core serializes \
                the event and calls update on the bridge. The bridge is FfiBridge, which \
                implements CoreBridge over the CoreFfi bindings that BoltFFI generates. \
                Without a BoltFFI config, the shell writes its own CoreBridge. The bytes \
                cross the FFI to CoreFfi in ffi.rs, which wraps Bridge<App> and Core<App>. \
                The requests come back up to Core. Core handles Render itself and gives \
                the other requests to EffectDispatcher. The dispatcher calls a method of \
                the shell's EffectHandler, which returns the output. The dispatcher \
                resolves the request through the bridge, and the new requests come back \
                up to Core.";
    let mut s = String::new();
    head(&mut s, W, H, title, desc);

    container(
        &mut s,
        Rect::new(15.0, 15.0, W - 30.0, shell_bottom - 15.0),
        Kind::Shell,
    );
    text(&mut s, 35.0, 44.0, "start", HEADING, "Shell");
    // The heading is on the right, clear of the FFI arrow on the left.
    let rust_area = Rect::new(15.0, rust_top, wide + 50.0, H - 15.0 - rust_top);
    container(&mut s, rust_area, Kind::Fn);
    text(
        &mut s,
        rust_area.x + rust_area.w - 18.0,
        rust_area.y + 27.0,
        "end",
        &faint(15.0, false),
        "Rust core (shared crate)",
    );

    // shell layers
    let ui = Rect::new(ax, r1, aw, r1h);
    lines_card(
        &mut s,
        ui,
        Kind::Shell,
        &[(TITLE, 16.0, "UI"), (SMALL, 12.0, "bound to core.view")],
    );
    let core = Rect::new(ax, r2, aw, r2h);
    lines_card(
        &mut s,
        core,
        Kind::Cmd,
        &[
            (CODE, 16.0, "Core"),
            (SMALL, 12.0, "holds the view and"),
            (SMALL, 12.0, "handles Render"),
        ],
    );
    let disp = Rect::new(bx, r2, bw, r2h);
    lines_card(
        &mut s,
        disp,
        Kind::Cmd,
        &[
            (CODE, 16.0, "EffectDispatcher"),
            (SMALL, 12.0, "resolves each request"),
        ],
    );
    let handler = Rect::new(cx, r2, cw, r2h);
    lines_card(
        &mut s,
        handler,
        Kind::Shell,
        &[
            (CODE, 16.0, "EffectHandler"),
            (SMALL, 12.0, "one method per variant"),
        ],
    );
    let bridge = Rect::new(ax, r3, wide, r3h);
    lines_card(
        &mut s,
        bridge,
        Kind::Cmd,
        &[
            (CODE, 16.0, "FfiBridge"),
            (
                SMALL,
                12.0,
                "implements CoreBridge (generated with a BoltFFI config)",
            ),
        ],
    );
    let bindings = Rect::new(ax, r4, wide, r4h);
    lines_card(
        &mut s,
        bindings,
        Kind::Cmd,
        &[
            (CODE, 16.0, "CoreFfi"),
            (SMALL, 12.0, "bindings that BoltFFI generates"),
        ],
    );

    // the Rust core: CoreFfi -> Bridge<App> -> Core<App>
    let rust_w = 150.0;
    let rust_gap = (wide - 3.0 * rust_w) / 2.0;
    let rust: Vec<Rect> = (0..3)
        .map(|i| Rect::new(ax + i as f64 * (rust_w + rust_gap), r5, rust_w, r5h))
        .collect();
    for (r, (name, sub)) in rust.iter().zip([
        ("CoreFfi", "ffi.rs"),
        ("Bridge<App>", "bytes to types"),
        ("Core<App>", "your App"),
    ]) {
        lines_card(
            &mut s,
            *r,
            Kind::Fn,
            &[(CODE, 16.0, name), (SMALL, 12.0, sub)],
        );
    }
    for pair in rust.windows(2) {
        h_arrow(&mut s, pair[0].x + pair[0].w + 2.0, r5 + r5h / 2.0, pair[1].x - 2.0);
    }

    // UI <-> Core
    let (xe, xv) = (ax + 55.0, ax + aw - 45.0);
    down(&mut s, xe, r1 + r1h, r2, Some(("update(event)", true)));
    arrow(
        &mut s,
        Path::new().m(xv, r2 - 2.0).v(r1 + r1h + 2.0),
        Arrow::Solid,
    );
    pill(&mut s, xv, (r1 + r1h + r2) / 2.0, "view", true, 12.5);

    // Core <-> bridge
    let (xu, xr) = (ax + 55.0, ax + aw - 45.0);
    down(&mut s, xu, r2 + r2h, r3, Some(("update(bytes)", true)));
    arrow(
        &mut s,
        Path::new().m(xr, r3 - 2.0).v(r2 + r2h + 2.0),
        Arrow::Solid,
    );
    pill(&mut s, xr, (r2 + r2h + r3) / 2.0, "requests", false, 12.5);

    // Core -> dispatcher -> handler, and back
    let mid = r2 + r2h / 2.0;
    h_arrow(&mut s, ax + aw + 2.0, mid, bx - 2.0);
    pill(&mut s, (ax + aw + bx) / 2.0, mid - 14.0, "dispatch", true, 12.5);
    h_arrow(&mut s, bx + bw + 2.0, mid - 12.0, cx - 2.0);
    pill(&mut s, (bx + bw + cx) / 2.0, mid - 26.0, "handler call", false, 12.5);
    h_arrow(&mut s, cx - 2.0, mid + 12.0, bx + bw + 2.0);
    pill(&mut s, (bx + bw + cx) / 2.0, mid + 26.0, "output", false, 12.5);

    // dispatcher -> bridge: resolve
    down(
        &mut s,
        bx + bw / 2.0,
        r2 + r2h,
        r3,
        Some(("resolve(id, bytes)", true)),
    );

    // bridge <-> bindings <-> Rust
    let xb = rust[0].cx();
    arrow(
        &mut s,
        Path::new().m(xb, r3 + r3h + 2.0).v(r4 - 2.0),
        Arrow::Both,
    );
    pill(&mut s, xb, (r3 + r3h + r4) / 2.0, "bytes", false, 12.5);
    arrow(
        &mut s,
        Path::new().m(xb, r4 + r4h + 2.0).v(r5 - 2.0),
        Arrow::Both,
    );
    pill(&mut s, xb, (shell_bottom + rust_top) / 2.0, "FFI", false, 12.5);

    legend(
        &mut s,
        cx,
        rust_top + 30.0,
        &[
            (Kind::Cmd, "generated"),
            (Kind::Shell, "the shell's own code"),
            (Kind::Fn, "the Rust shared crate"),
        ],
    );

    assert!(r5 + r5h + 18.0 + 15.0 <= H, "{}", r5 + r5h + 33.0);
    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// Testing with fakes (part-2/mocking_layers.svg)
// ---------------------------------------------------------------------------

/// The layers of an app at run time, next to the layers in a test that uses
/// fakes. The core is the same in both columns. The transport layers are not
/// in the test, and a simplified version of the server logic runs in the test
/// process.
fn mocking_layers() -> String {
    const W: f64 = 800.0;
    let (lx, rx, cw) = (30.0, 500.0, 270.0); // column lefts and the column width
    let (rh, gap) = (56.0, 14.0); // card height and the gap between cards
    let top = 64.0;
    let row = |i: usize| top + i as f64 * (rh + gap);
    let gx = (lx + cw + rx) / 2.0; // the middle of the gap between the columns

    // The transport group: a heading above three dashed cards.
    let pad = 12.0;
    let group_top = row(2);
    let heading_room = 36.0;
    let transport_y = |i: usize| group_top + heading_room + i as f64 * (rh + gap);
    let group_h = heading_room + 3.0 * rh + 2.0 * gap + pad;
    let server_y = group_top + group_h + gap;
    let h = server_y + rh + 20.0;

    let title = "Crux: the app layers at run time and in a test";
    let desc = "Two columns of layers. At runtime, from the top: the app core, the \
                shell's effect handler, an HTTP client, the network, an HTTP server \
                and the server logic. In a test, from the top: the same app core, an \
                effect handler for the test, and a simplified version of the server \
                logic. An arrow labelled unchanged goes from the app core at run time \
                to the app core in the test. An arrow labelled test version goes from \
                the shell's effect handler to the test's effect handler. The HTTP \
                client, the network and the HTTP server have dashed outlines, in a \
                group labelled not in tests. An arrow labelled in-process fake goes \
                from the server logic to the simplified server logic, which runs in \
                the test process.";
    let mut s = String::new();
    head(&mut s, W, h, title, desc);

    text(&mut s, lx + cw / 2.0, 40.0, "middle", HEADING, "At runtime");
    text(&mut s, rx + cw / 2.0, 40.0, "middle", HEADING, "In tests");

    // At run time
    let core = Rect::new(lx, row(0), cw, rh);
    lines_card(&mut s, core, Kind::Fn, &[(TITLE, 16.0, "App core")]);
    let handler = Rect::new(lx, row(1), cw, rh);
    lines_card(
        &mut s,
        handler,
        Kind::Shell,
        &[(TITLE, 16.0, "Effect handler (shell)")],
    );
    group(
        &mut s,
        Rect::new(lx - pad, group_top, cw + 2.0 * pad, group_h),
        Kind::World,
        "Transport (not in tests)",
        false,
    );
    for (i, name) in ["HTTP client", "Network", "HTTP server"].into_iter().enumerate() {
        let r = Rect::new(lx, transport_y(i), cw, rh);
        bold_dashed_card(&mut s, r);
        text(&mut s, r.cx(), r.cy() + 5.5, "middle", TITLE, name);
    }
    let server = Rect::new(lx, server_y, cw, rh);
    lines_card(&mut s, server, Kind::Model, &[(TITLE, 16.0, "Server logic")]);

    // In tests: the same rows as the core and the handler, and the fake
    // directly under the handler.
    let test_core = Rect::new(rx, row(0), cw, rh);
    lines_card(&mut s, test_core, Kind::Fn, &[(TITLE, 16.0, "App core")]);
    let test_handler = Rect::new(rx, row(1), cw, rh);
    lines_card(
        &mut s,
        test_handler,
        Kind::Shell,
        &[(TITLE, 16.0, "Effect handler (test)")],
    );
    let fake = Rect::new(rx, row(2), cw, rh);
    lines_card(
        &mut s,
        fake,
        Kind::Model,
        &[(TITLE, 16.0, "Simplified server logic")],
    );

    // Each arrow has a label that fits in the gap between the columns.
    let fits = |label: &str| {
        assert!(
            text_width(label, 12.5) + 16.0 + 12.0 <= rx - (lx + cw),
            "{label:?} does not fit between the columns"
        );
    };
    for (r, label) in [(core, "unchanged"), (handler, "test version")] {
        fits(label);
        h_arrow(&mut s, lx + cw + 2.0, r.cy(), rx - 2.0);
        pill(&mut s, gx, r.cy() - 14.0, label, false, 12.5);
    }
    // Server logic -> simplified server logic: right, up the gap, then right.
    let label = "in-process fake";
    fits(label);
    let (y0, y1, rad) = (server.cy(), fake.cy(), 16.0);
    arrow(
        &mut s,
        Path::new()
            .m(lx + cw + 2.0, y0)
            .h(gx - rad)
            .q(gx, y0, gx, y0 - rad)
            .v(y1 + rad)
            .q(gx, y1, gx + rad, y1)
            .h(rx - 2.0),
        Arrow::Solid,
    );
    pill(&mut s, gx, (y0 + y1) / 2.0, label, false, 12.5);

    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// The weather app's state machines (part-2/app_lifecycle.svg and
// part-2/nested_state_machines.svg)
// ---------------------------------------------------------------------------

/// A card title or a label on a pill, and whether it is code.
type Label = (&'static str, bool);

/// A line of text from `x`, in parts. A part that is code is monospace.
fn mixed_text(out: &mut String, x: f64, y: f64, attrs: &str, parts: &[Label]) {
    let mut content = String::new();
    for &(part, mono) in parts {
        if mono {
            write!(content, r#"<tspan class="m">{}</tspan>"#, escape(part)).unwrap();
        } else {
            content.push_str(&escape(part));
        }
    }
    let (x, y) = (n(x), n(y));
    emit!(
        out,
        r#"<text x="{x}" y="{y}" text-anchor="start" {attrs} xml:space="preserve">{content}</text>"#
    );
}

/// The length of an arrowhead: the marker is 8 stroke widths long.
const HEAD: f64 = 16.0;
/// The plain line between a pill or a bend and an arrowhead.
const CLEAR: f64 = 24.0;
/// The height of a pill with two lines of 12.5px text. Rows of pills use it
/// to leave room for a path that does not fit on one line.
const PILL2_H: f64 = 38.0;

/// The lines of a Rust path on a pill. The path is on one line if it fits in
/// `max_w`. If not, it goes on two lines: the type with `::`, then the
/// variant.
fn path_lines(path: &str, max_w: f64) -> Vec<&str> {
    if text_width(path, 12.5) + 16.0 <= max_w {
        return vec![path];
    }
    let split = path.rfind("::").expect("a path with ::") + 2;
    let lines = vec![&path[..split], &path[split..]];
    assert!(
        lines.iter().all(|l| text_width(l, 12.5) + 16.0 <= max_w),
        "{path:?} does not fit in {max_w}px"
    );
    lines
}

/// The height of a pill with `count` lines of 12.5px text. A pill with one
/// line has the height of a `pill`.
fn pill_height(count: usize) -> f64 {
    if count == 1 {
        12.5 + 10.0
    } else {
        count as f64 * 15.0 + 8.0
    }
}

/// A pill with lines of monospace 12.5px text, centred on (`cx`, `cy`). The
/// function returns the box of the pill.
fn code_pill(out: &mut String, cx: f64, cy: f64, lines: &[&str]) -> Bounds {
    let w = lines.iter().map(|l| text_width(l, 12.5)).fold(0.0, f64::max) + 16.0;
    let h = pill_height(lines.len());
    let b = [cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0];
    emit!(
        out,
        r#"<rect x="{}" y="{}" width="{}" height="{}" rx="11" class="pill"/>"#,
        n(b[0]),
        n(b[1]),
        n(w),
        n(h)
    );
    let first = -(lines.len() as f64 - 1.0) / 2.0;
    for (i, line) in lines.iter().enumerate() {
        let y = cy + (first + i as f64) * 15.0 + 12.5 * 0.36;
        text(out, cx, y, "middle", r#"class="l m""#, line);
    }
    b
}

/// The widest pill that each of the vertical lines at `xs` (from left to
/// right) can have in one row. A pill stops 6px before the middle of the gap
/// to the next line, and 30px inside the edges of a diagram `w` wide.
fn pill_widths(xs: &[f64], w: f64) -> Vec<f64> {
    (0..xs.len())
        .map(|i| {
            let left = if i == 0 { xs[0] - 30.0 } else { (xs[i] - xs[i - 1]) / 2.0 - 6.0 };
            let right = match xs.get(i + 1) {
                Some(next) => (next - xs[i]) / 2.0 - 6.0,
                None => w - 30.0 - xs[i],
            };
            2.0 * left.min(right)
        })
        .collect()
}

/// A path pill in the middle of a vertical run of line from `from` to an
/// arrowhead at `tip`, in either direction. The function panics unless the
/// pill leaves `CLEAR` to the start of the run and to the arrowhead.
fn v_path_pill(out: &mut String, x: f64, from: f64, tip: f64, path: &str, max_w: f64) -> Bounds {
    let lines = path_lines(path, max_w);
    let mid = (from + tip) / 2.0;
    let half = (tip - from).abs() / 2.0 - pill_height(lines.len()) / 2.0;
    assert!(
        half - HEAD >= CLEAR,
        "the pill {path:?} is too close to the end of its line"
    );
    code_pill(out, x, mid, &lines)
}

/// Panic if two pills overlap or come closer than 8px.
fn assert_apart(pills: &[Bounds]) {
    for (i, a) in pills.iter().enumerate() {
        for b in &pills[i + 1..] {
            let a = [a[0] - 8.0, a[1] - 8.0, a[2] + 8.0, a[3] + 8.0];
            assert!(disjoint(&a, b), "the pills {a:?} and {b:?} are too close");
        }
    }
}

/// A state of the lifecycle: the variant name and a short description.
fn state(out: &mut String, r: Rect, name: &str, sub: &str) {
    lines_card(out, r, Kind::Model, &[(CODE, 16.0, name), (SMALL, 12.0, sub)]);
}

/// The phases of the weather app's `Model`, and the transitions between them.
/// Each label is the event or the transition that moves the app to the next
/// phase.
fn app_lifecycle() -> String {
    const W: f64 = 960.0;
    let (cw, ch) = (240.0, 56.0); // the state cards
    // between the rows that a vertical arrow joins
    let gap = 2.0 * (HEAD + CLEAR) + PILL2_H + 4.0;
    let rad = 16.0; // the radius of a bend
    let spine = W / 2.0;
    let uninit = Rect::new(spine - cw / 2.0, 60.0, cw, ch);
    let init = Rect::new(spine - cw / 2.0, uninit.y + ch + gap, cw, ch);
    let row2 = init.cy() + rad + 2.0 + HEAD + CLEAR + PILL2_H + CLEAR + HEAD;
    let onboard = Rect::new(80.0, row2, cw, ch);
    let active = Rect::new(W - 80.0 - cw, row2, cw, ch);
    let failed = Rect::new(onboard.x, onboard.y + ch + gap, cw, ch);
    let h = failed.y + ch + 33.0;

    let title = "Crux: the weather app's lifecycle";
    let desc = "The five states of the weather app's Model, with an arrow for each \
                transition. Event::Start moves the app from Uninitialized to \
                Initializing. Initializing goes to Onboard with \
                InitializingTransition::Onboard if no API key is stored, and to Active \
                with InitializingTransition::Active if the key is stored. Onboard goes to \
                Active with OnboardTransition::Active, and to Failed with \
                OnboardTransition::Failed. Active goes back to Onboard with \
                ActiveTransition::Unauthorized or ActiveTransition::ResetApiKey. Failed \
                holds the message of an unrecoverable error, and the app stays in \
                Failed.";
    let mut s = String::new();
    head(&mut s, W, h, title, desc);
    group(
        &mut s,
        Rect::new(15.0, 15.0, W - 30.0, h - 30.0),
        Kind::Model,
        "Model",
        true,
    );

    state(&mut s, uninit, "Uninitialized", "waits for the shell");
    state(&mut s, init, "Initializing", "loads the key and favourites");
    state(&mut s, onboard, "Onboard", "asks for an API key");
    state(&mut s, active, "Active", "home and favourites");
    state(&mut s, failed, "Failed", "an unrecoverable error");

    let mut pills = Vec::new();
    // A pill on a vertical line can be as wide as a card.
    let max_w = cw;
    // Uninitialized -> Initializing
    let (from, tip) = (uninit.y + ch + 2.0, init.y - 2.0);
    arrow(&mut s, Path::new().m(spine, from).v(tip), Arrow::Solid);
    pills.push(v_path_pill(&mut s, spine, from, tip, "Event::Start", max_w));

    // Initializing -> Onboard and Initializing -> Active: out of the sides of
    // Initializing, then down into the tops of the two states.
    for (target, side, dir, label) in [
        (onboard, init.x - 2.0, -1.0, "InitializingTransition::Onboard"),
        (active, init.x + cw + 2.0, 1.0, "InitializingTransition::Active"),
    ] {
        let x = target.cx();
        let (y, tip) = (init.cy(), target.y - 2.0);
        arrow(
            &mut s,
            Path::new()
                .m(side, y)
                .h(x - dir * rad)
                .q(x, y, x, y + rad)
                .v(tip),
            Arrow::Solid,
        );
        pills.push(v_path_pill(&mut s, x, y + rad, tip, label, max_w));
    }

    // Onboard -> Active above the middle of the cards, and Active -> Onboard
    // below it. Each pill touches its line: above the upper line and below
    // the lower line. One pill names both transitions from Active.
    let (left, right) = (onboard.x + cw + 2.0, active.x - 2.0);
    let (upper, lower) = (onboard.cy() - 12.0, onboard.cy() + 12.0);
    let between = right - left - 2.0 * (HEAD + CLEAR);
    h_arrow(&mut s, left, upper, right);
    let lines = path_lines("OnboardTransition::Active", between);
    let cy = upper - 4.0 - pill_height(lines.len()) / 2.0;
    pills.push(code_pill(&mut s, spine, cy, &lines));
    h_arrow(&mut s, right, lower, left);
    let lines = ["ActiveTransition::", "Unauthorized", "ResetApiKey"];
    let cy = lower + 4.0 + pill_height(lines.len()) / 2.0;
    pills.push(code_pill(&mut s, spine, cy, &lines));
    // The pills between the cards keep a clear run to each arrowhead.
    for b in &pills[pills.len() - 2..] {
        assert!(b[0] - (left + HEAD) >= CLEAR && (right - HEAD) - b[2] >= CLEAR);
    }

    // Onboard -> Failed
    let (from, tip) = (onboard.y + ch + 2.0, failed.y - 2.0);
    arrow(&mut s, Path::new().m(onboard.cx(), from).v(tip), Arrow::Solid);
    pills.push(v_path_pill(
        &mut s,
        onboard.cx(),
        from,
        tip,
        "OnboardTransition::Failed",
        max_w,
    ));
    // The three-line pill stays above Failed.
    assert!(pills[4][3] + 12.0 < failed.y);

    assert_apart(&pills);
    s.push_str("</svg>\n");
    s
}

/// The `Active` branch of the weather app's model tree. Events go down from
/// each parent to its child on solid lines, and transitions come back up on
/// dashed lines, until `Model` swaps `Active` for `Onboard`.
fn nested_state_machines() -> String {
    const W: f64 = 960.0;
    let ch = 56.0;
    let (xe, xt) = (240.0, 416.0); // the event and transition lines of the chain
    let gap = 2.0 * (HEAD + CLEAR) + PILL2_H + 4.0;
    let (chain_x, chain_w) = (160.0, 320.0);
    let active = Rect::new(chain_x, 60.0, chain_w, ch);
    let onboard = Rect::new(740.0, 60.0, 190.0, ch);
    // The arrow into the bottom of Active crosses the bottom of the group on a
    // clear run of line, below its arrowhead.
    let group_bottom = active.y + ch + 2.0 + HEAD + CLEAR + 2.0;
    let active_model = Rect::new(chain_x, group_bottom + gap, chain_w, ch);
    let home = Rect::new(30.0, active_model.y + ch + gap, W - 60.0, ch);
    let local = Rect::new(30.0, home.y + ch + gap, 450.0, ch);
    let favs = Rect::new(520.0, local.y, W - 30.0 - 520.0, ch);
    let h = local.y + ch + 22.0;

    let title = "Crux: events go down the model tree, and transitions come back up";
    let desc = "The Active branch of the weather app's model tree. The Model group holds \
                the Active and Onboard states. Active holds an ActiveModel. Its screen \
                is Screen::Home, which holds a HomeScreen. The HomeScreen holds two \
                children, LocalWeather and Vec<FavoriteWeather>. Solid arrows take \
                events down from each parent to its child: Event::Active, \
                ActiveEvent::Home, HomeEvent::Local and HomeEvent::FavoritesWeather. \
                Dashed arrows take transitions back up in the Outcome that update \
                returns: LocalWeatherTransition::Unauthorized and \
                FavoriteWeatherTransition::Unauthorized go to HomeScreen, \
                HomeTransition::ApiKeyRejected goes to ActiveModel, and \
                ActiveTransition::Unauthorized goes to Active. Then update_active \
                changes the Model from Active back to Onboard. Each parent uses map_event to \
                wrap the events of its child in its own variant.";
    let mut s = String::new();
    head(&mut s, W, h, title, desc);
    group(
        &mut s,
        Rect::new(15.0, 15.0, W - 30.0, group_bottom - 15.0),
        Kind::Model,
        "Model",
        true,
    );
    state(&mut s, active, "Active", "home and favourites");
    state(&mut s, onboard, "Onboard", "asks for an API key");

    let machine = |s: &mut String, r: Rect, name: &str, sub: (&str, &str)| {
        lines_card(s, r, Kind::Model, &[(CODE, 16.0, name), (sub.0, 12.0, sub.1)]);
    };
    machine(
        &mut s,
        active_model,
        "ActiveModel",
        (LINE_SMALL_CODE, "screen: Screen::Home"),
    );
    machine(
        &mut s,
        home,
        "HomeScreen",
        (SMALL, "two sub-workflows that run in parallel"),
    );
    machine(
        &mut s,
        local,
        "LocalWeather",
        (SMALL, "the weather at the current location"),
    );
    machine(
        &mut s,
        favs,
        "Vec<FavoriteWeather>",
        (SMALL, "the weather for each favourite"),
    );

    // Active -> Onboard, when update_active gets the transition.
    let (left, right) = (active.x + chain_w + 2.0, onboard.x - 2.0);
    h_arrow(&mut s, left, active.cy(), right);
    let label = "back to onboarding";
    let w = text_width(label, 12.5) + 16.0;
    let mid = (left + right) / 2.0;
    assert!(mid - w / 2.0 - left >= CLEAR && right - HEAD - (mid + w / 2.0) >= CLEAR);
    pill(&mut s, mid, active.cy() - 14.0, label, false, 12.5);

    // Each link: the parent, the child, the x of the event line and of the
    // transition line, the event and the transition.
    let links = [
        (active, active_model, xe, xt, "Event::Active", "ActiveTransition::Unauthorized"),
        (
            active_model,
            home,
            xe,
            xt,
            "ActiveEvent::Home",
            "HomeTransition::ApiKeyRejected",
        ),
        (
            home,
            local,
            120.0,
            340.0,
            "HomeEvent::Local",
            "LocalWeatherTransition::Unauthorized",
        ),
        (
            home,
            favs,
            560.0,
            800.0,
            "HomeEvent::FavoritesWeather",
            "FavoriteWeatherTransition::Unauthorized",
        ),
    ];
    // The widths that the pills of each row can have. The last two links
    // share a row.
    let chain_row = pill_widths(&[xe, xt], W);
    let child_row = pill_widths(&[120.0, 340.0, 560.0, 800.0], W);
    let max_ws = [
        chain_row.clone(),
        chain_row,
        child_row[..2].to_vec(),
        child_row[2..].to_vec(),
    ];
    let mut pills = Vec::new();
    for (i, &(parent, child, x_event, x_trans, event, transition)) in links.iter().enumerate() {
        for x in [x_event, x_trans] {
            assert!(x > parent.x && x < parent.x + parent.w && x > child.x && x < child.x + child.w);
        }
        let (top, bottom) = (parent.y + ch + 2.0, child.y - 2.0);
        // The first link crosses the bottom of the group, so its pills sit
        // below the group.
        let from = if i == 0 { group_bottom } else { top };
        arrow(&mut s, Path::new().m(x_event, top).v(bottom), Arrow::Solid);
        let (max_event, max_trans) = (max_ws[i][0], max_ws[i][1]);
        pills.push(v_path_pill(&mut s, x_event, from, bottom, event, max_event));
        arrow(&mut s, Path::new().m(x_trans, bottom).v(top), Arrow::Dashed);
        let b = v_path_pill(&mut s, x_trans, bottom, from, transition, max_trans);
        // The pill of the up arrow keeps a clear run to its arrowhead.
        assert!(b[1] - (top + HEAD) >= CLEAR);
        pills.push(b);
    }
    assert_apart(&pills);

    // The legend, to the right of ActiveModel.
    let (lx, ly) = (540.0, active_model.y + 14.0);
    let attrs = faint(13.0, false);
    for (i, (kind, parts)) in [
        (Arrow::Solid, &[("events go down to a child", false)][..]),
        (
            Arrow::Dashed,
            &[("transitions come back up in an ", false), ("Outcome", true)][..],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let y = ly + i as f64 * 28.0;
        arrow(&mut s, Path::new().m(lx, y - 4.0).h(lx + 44.0), kind);
        mixed_text(&mut s, lx + 58.0, y, &attrs, parts);
    }
    mixed_text(
        &mut s,
        lx,
        ly + 56.0,
        &attrs,
        &[("map_event", true), (" wraps a child's events in a variant", false)],
    );
    // The legend is clear of the pills and of HomeScreen.
    let legend = [lx - 8.0, ly - 14.0, W - 20.0, ly + 62.0];
    assert!(pills.iter().all(|b| disjoint(&legend, b)) && legend[3] + 12.0 < home.y);

    s.push_str("</svg>\n");
    s
}

// ---------------------------------------------------------------------------
// The effect router (part-3/effect_router*.svg)
// ---------------------------------------------------------------------------

/// Where a route sends its effects in the shell.
struct ShellEnd {
    name: Label,
    sub: &'static str,
    /// The label on the arrow from the route to the shell.
    request: Label,
}

/// One route in an effect router diagram.
struct Route {
    name: Label,
    sub: &'static str,
    /// The label on the arrow from the routing closure, if any.
    effects: Option<Label>,
    /// The handler in the shell, or `None` for a Rust handler that resolves
    /// its requests itself.
    shell: Option<ShellEnd>,
    /// The label on the arrow that takes the output back to the core.
    output: Label,
}

/// The strings and the width of one effect router diagram.
struct Router {
    width: f64,
    title: &'static str,
    desc: &'static str,
    core_sub: &'static str,
    /// The label on the arrow from the core to the routing closure.
    effects: Label,
    /// The heading of the routes group, in parts that are code or prose.
    routes_heading: &'static [Label],
    shell_heading: &'static str,
    /// The left of the routes group. The space to its left holds the arrow
    /// that takes the outputs back to the core.
    routes_x: f64,
    card_w: f64,
    /// The first route is the Rust handler, and the others go to the shell.
    routes: &'static [Route],
}

/// The height of a pill with 12.5px text.
const PILL_H: f64 = 22.5;

/// Panic unless a pill centred at `y` on a vertical line from `y0` down to
/// an arrowhead at `y1` leaves a clear run of line on each side of it.
fn assert_clear_run(y: f64, y0: f64, y1: f64) {
    assert!(
        y - PILL_H / 2.0 - y0 >= CLEAR && y1 - HEAD - (y + PILL_H / 2.0) >= CLEAR,
        "the pill at {y} is too close to the end of its line"
    );
}

/// An effect router: the core and the routing closure, the routes below
/// them, and the shell under the routes. Every output goes back to the core
/// along one line on the left, and the core sends the follow-up effects to
/// the routing closure again.
fn effect_router(p: &Router) -> String {
    let w = p.width;
    let bus = 100.0; // the x of the line that takes the outputs back to the core
    let rad = 16.0; // the radius of the corners of the lines
    let card_h = 64.0;
    let router_y = 58.0; // the EffectRouter group
    let top = router_y + 42.0; // the core and the routing closure
    // The arrow into the bottom of the core crosses the bottom of the group on
    // a clear run of line, below its arrowhead.
    let router_h = top + card_h + 2.0 + HEAD + CLEAR - router_y;
    let routes_y = router_y + router_h + 36.0; // the routes group
    let branch_y = routes_y - 18.0; // where the arrows from the closure divide
    // A pill centred on the line after the bend, with a clear run on each side.
    let lane_y = branch_y + rad + 2.0 * (CLEAR + HEAD) + PILL_H + 2.0; // the route cards
    let routes_h = lane_y + card_h + 44.0 - routes_y;
    let rust_bottom = routes_y + routes_h + 16.0;
    let shell_y = rust_bottom + 40.0;
    // As far below the top of the shell as the route cards are above the
    // bottom of the Rust area, so that a pill in the gap is centred on its line.
    let dest_y = shell_y + (rust_bottom - (lane_y + card_h)); // the shell cards
    let bus_y = dest_y + card_h + 54.0; // the shell outputs join here
    let h = bus_y + 20.0 + 15.0;

    assert!(
        p.routes[0].shell.is_none(),
        "the first route is the Rust handler"
    );
    assert!(p.routes[1..].iter().all(|r| r.shell.is_some()));
    let n = p.routes.len() as f64;
    let routes = Rect::new(p.routes_x, routes_y, w - 35.0 - p.routes_x, routes_h);
    let gap = (routes.w - 40.0 - n * p.card_w) / (n - 1.0);
    let lanes: Vec<Rect> = (0..p.routes.len())
        .map(|i| {
            Rect::new(
                routes.x + 20.0 + i as f64 * (p.card_w + gap),
                lane_y,
                p.card_w,
                card_h,
            )
        })
        .collect();
    let mid = (lanes[0].cx() + lanes[lanes.len() - 1].cx()) / 2.0;

    let mut s = String::new();
    head(&mut s, w, h, p.title, p.desc);
    let attrs = |mono: bool| if mono { CODE } else { TITLE };

    // Rust
    container(
        &mut s,
        Rect::new(15.0, 15.0, w - 30.0, rust_bottom - 15.0),
        Kind::Fn,
    );
    text(
        &mut s,
        35.0,
        42.0,
        "start",
        &faint(15.0, false),
        "Rust (shared crate)",
    );
    group(
        &mut s,
        Rect::new(35.0, router_y, w - 70.0, router_h),
        Kind::Cmd,
        "EffectRouter",
        true,
    );
    let core = Rect::new(55.0, top, 196.0, card_h);
    lines_card(
        &mut s,
        core,
        Kind::Fn,
        &[(CODE, 16.0, "Core"), (SMALL, 12.0, p.core_sub)],
    );
    let closure = Rect::new(mid - 120.0, top, 240.0, card_h);
    lines_card(
        &mut s,
        closure,
        Kind::Cmd,
        &[
            (TITLE, 16.0, "routing closure"),
            (SMALL, 12.0, "sends each effect to a route"),
        ],
    );
    let (label, mono) = p.effects;
    let x0 = core.x + core.w;
    // The pill is above the line, but clear of the arrowhead as well.
    let pill_w = text_width(label, 12.5) + 16.0;
    assert!(
        (closure.x - 2.0 - x0 - pill_w) / 2.0 - HEAD >= CLEAR,
        "{label:?} is too close to the arrowhead"
    );
    h_arrow(&mut s, x0 + 2.0, core.cy(), closure.x - 2.0);
    pill(
        &mut s,
        (x0 + closure.x) / 2.0,
        core.cy() - 14.0,
        label,
        mono,
        12.5,
    );

    // The routes. The heading is at the bottom left, under the Rust
    // handler, clear of the arrows from the routing closure.
    container(&mut s, routes, Kind::Model);
    mixed_text(
        &mut s,
        routes.x + 18.0,
        routes.y + routes.h - 14.0,
        &faint(15.0, false),
        p.routes_heading,
    );
    let from = closure.y + closure.h + 2.0;
    for (route, lane) in p.routes.iter().zip(&lanes) {
        let (name, mono) = route.name;
        lines_card(
            &mut s,
            *lane,
            Kind::Model,
            &[(attrs(mono), 16.0, name), (SMALL, 12.0, route.sub)],
        );
        let x = lane.cx();
        let path = if x == mid {
            Path::new().m(mid, from).v(lane_y - 2.0)
        } else {
            let dir = (x - mid).signum();
            Path::new()
                .m(mid, from)
                .v(branch_y - rad)
                .q(mid, branch_y, mid + dir * rad, branch_y)
                .h(x - dir * rad)
                .q(x, branch_y, x, branch_y + rad)
                .v(lane_y - 2.0)
        };
        arrow(&mut s, path, Arrow::Solid);
        if let Some((label, mono)) = route.effects {
            let start = if x == mid { from } else { branch_y + rad };
            let y = (branch_y + rad + lane_y - 2.0) / 2.0;
            assert_clear_run(y, start, lane_y - 2.0);
            pill(&mut s, x, y, label, mono, 12.5);
        }
    }

    // The shell
    container(
        &mut s,
        Rect::new(15.0, shell_y, w - 30.0, h - 15.0 - shell_y),
        Kind::Shell,
    );
    text(
        &mut s,
        bus + 30.0,
        shell_y + 27.0,
        "start",
        &faint(15.0, false),
        p.shell_heading,
    );
    let core_bottom = core.y + core.h + 2.0;
    for (route, lane) in p.routes.iter().zip(&lanes) {
        let x = lane.cx();
        let (label, mono) = route.output;
        let Some(end) = &route.shell else {
            // The Rust handler resolves its requests itself.
            arrow(
                &mut s,
                Path::new()
                    .m(lane.x - 2.0, lane.cy())
                    .h(bus + rad)
                    .q(bus, lane.cy(), bus, lane.cy() - rad)
                    .v(core_bottom),
                Arrow::Solid,
            );
            let room = routes.x - (bus + rad);
            assert!(text_width(label, 12.5) + 16.0 + 12.0 <= room, "{label:?}");
            pill(
                &mut s,
                (bus + rad + routes.x) / 2.0,
                lane.cy(),
                label,
                mono,
                12.5,
            );
            continue;
        };
        let dest = Rect::new(lane.x, dest_y, lane.w, card_h);
        let (name, name_mono) = end.name;
        lines_card(
            &mut s,
            dest,
            Kind::Shell,
            &[(attrs(name_mono), 16.0, name), (SMALL, 12.0, end.sub)],
        );
        let (request, request_mono) = end.request;
        down(&mut s, x, lane.y + lane.h, dest_y, None);
        let y = (rust_bottom + shell_y) / 2.0;
        assert!(y == (lane.y + lane.h + dest_y) / 2.0, "the pill is off centre");
        assert_clear_run(y, lane.y + lane.h + 2.0, dest_y - 2.0);
        pill(
            &mut s,
            x,
            y,
            request,
            request_mono,
            12.5,
        );
        arrow(
            &mut s,
            Path::new()
                .m(x, dest_y + card_h + 2.0)
                .v(bus_y - rad)
                .q(x, bus_y, x - rad, bus_y)
                .h(bus + rad)
                .q(bus, bus_y, bus, bus_y - rad)
                .v(core_bottom),
            Arrow::Solid,
        );
        pill(
            &mut s,
            x,
            (dest_y + card_h + bus_y) / 2.0,
            label,
            mono,
            12.5,
        );
    }

    s.push_str("</svg>\n");
    s
}

/// The parts of the effect router, without the counter's names.
const EFFECT_ROUTER: Router = Router {
    width: 1100.0,
    title: "Crux: the effect router",
    desc: "The Rust shared crate holds an EffectRouter, which holds the Core and a \
           routing closure. The Core runs the app, and the app does not change. The Core \
           gives each effect and each follow-up effect to the routing closure. The \
           routing closure sends each effect to a route of your own. A Rust handler \
           resolves its requests itself, next to the core. The Serialized lane sends \
           bytes to the shell's EffectHandler, which is the usual serialized interface. \
           The Parked lane sends the typed operation to a custom FFI of your own design. \
           Each output goes back to the Core along one line, and the Core sends the \
           follow-up effects to the routing closure again.",
    core_sub: "runs the app, unchanged",
    effects: ("effects and follow-ups", false),
    routes_heading: &[("Your Routes type", false)],
    shell_heading: "Shell",
    routes_x: 220.0,
    card_w: 215.0,
    routes: &[
        Route {
            name: ("Rust handler", false),
            sub: "runs alongside the core",
            effects: None,
            shell: None,
            output: ("output", false),
        },
        Route {
            name: ("Serialized", true),
            sub: "serializes each effect",
            effects: None,
            shell: Some(ShellEnd {
                name: ("EffectHandler", true),
                sub: "the serialized interface",
                request: ("bytes", false),
            }),
            output: ("serialized output", false),
        },
        Route {
            name: ("Parked", true),
            sub: "parks the typed request",
            effects: None,
            shell: Some(ShellEnd {
                name: ("Custom FFI", false),
                sub: "your own design",
                request: ("typed operation", false),
            }),
            output: ("typed output", false),
        },
    ],
};

/// The effect router in the counter-routing example.
const EFFECT_ROUTER_COUNTER: Router = Router {
    width: 920.0,
    title: "Crux: the effect router in the counter",
    desc: "The Rust shared crate holds the EffectRouter and the EffectRoutes type, \
           which implements Routes. The EffectRouter holds the Core, which runs the \
           app, and the routing closure. The Core gives its effects to the routing \
           closure. The closure sends Random to the RngHandler, which runs on its own \
           thread, and everything else to the Serialized lane. The Serialized lane \
           sends bytes to the shell's EffectHandler through the CruxShell callback. \
           The shell is in Swift or Kotlin. The RngHandler sends a \
           RandomNumber back to the Core, and the shell sends the serialized output \
           back to the Core.",
    core_sub: "runs the app",
    effects: ("effects", false),
    routes_heading: &[("EffectRoutes", true), (" (our Routes impl)", false)],
    shell_heading: "Shell (Swift, Kotlin)",
    routes_x: 250.0,
    card_w: 250.0,
    routes: &[
        Route {
            name: ("RngHandler", true),
            sub: "runs on its own thread",
            effects: Some(("Random", true)),
            shell: None,
            output: ("RandomNumber", true),
        },
        Route {
            name: ("Serialized", true),
            sub: "serializes each effect",
            effects: Some(("everything else", false)),
            shell: Some(ShellEnd {
                name: ("EffectHandler", true),
                sub: "the serialized interface",
                request: ("bytes, via CruxShell callback", false),
            }),
            output: ("serialized output", false),
        },
    ],
};

fn main() -> std::io::Result<()> {
    // Under `-Zscript`, CARGO_MANIFEST_DIR is the directory of this script, so
    // the default output directory does not depend on the current directory.
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/../src", env!("CARGO_MANIFEST_DIR")));
    std::fs::write(format!("{out}/command_overview.svg"), round_trip(true))?;
    std::fs::write(format!("{out}/architecture.svg"), round_trip(false))?;
    std::fs::write(format!("{out}/crux.svg"), ports())?;
    for p in &CODEGEN {
        std::fs::write(format!("{out}/{}", p.file), codegen(p))?;
    }
    std::fs::write(format!("{out}/shell_leptos.svg"), leptos())?;
    std::fs::write(
        format!("{out}/part-4/effect_life_cycle.svg"),
        effect_life_cycle(),
    )?;
    std::fs::write(
        format!("{out}/part-4/request_nesting.svg"),
        request_nesting(),
    )?;
    std::fs::write(format!("{out}/part-4/generated_core.svg"), generated_core())?;
    std::fs::write(format!("{out}/part-2/mocking_layers.svg"), mocking_layers())?;
    std::fs::write(format!("{out}/part-2/app_lifecycle.svg"), app_lifecycle())?;
    std::fs::write(
        format!("{out}/part-2/nested_state_machines.svg"),
        nested_state_machines(),
    )?;
    std::fs::write(
        format!("{out}/part-3/effect_router.svg"),
        effect_router(&EFFECT_ROUTER),
    )?;
    std::fs::write(
        format!("{out}/part-3/effect_router_counter.svg"),
        effect_router(&EFFECT_ROUTER_COUNTER),
    )?;
    Ok(())
}
