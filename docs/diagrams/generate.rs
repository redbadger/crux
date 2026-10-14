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
    Ok(())
}
