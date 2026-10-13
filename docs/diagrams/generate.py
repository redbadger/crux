"""Generate the Crux book diagrams as hand-written SVG.

This script writes three files to docs/src:

- command_overview.svg, the round trip between the shell and the core
- architecture.svg, the same diagram for the README
- crux.svg, the driving and driven sides of the shell around the core

Do not edit the SVG files directly. Change this script and run it again:

    just diagrams        (from docs/)
    python3 diagrams/generate.py [OUT_DIR]

The SVG files use system fonts and a transparent background, so that they show
on light and dark themes in the book and on GitHub.
"""
import math
import sys
from pathlib import Path
from xml.sax.saxutils import escape

OUT = sys.argv[1] if len(sys.argv) > 1 else str(Path(__file__).resolve().parent.parent / "src")

LINE = "#8a8a8a"
INK = "#1a1a1a"
SANS = 'system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif'
MONO = 'ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace'

STYLE = f"""<style>
text {{ font-family: {SANS}; fill: {INK}; }}
.m {{ font-family: {MONO}; }}
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
</style>"""

DEFS = f"""<defs>
<marker id="a" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
<path d="M0,0 L10,5 L0,10 z" fill="{LINE}"/>
</marker>
</defs>"""

COL = {
    "shell": ("#bfdbfe", "#3b82f6"),
    "fn": ("#bbf7d0", "#22c55e"),
    "model": ("#fde68a", "#d97706"),
    "cmd": ("#ddd6fe", "#8b5cf6"),
    "effect": ("#fecaca", "#ef4444"),
    "event": ("#bfdbfe", "#3b82f6"),
    "world": ("#e5e7eb", "#9ca3af"),
}


def head(w, h, title, desc):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" '
        f'width="{w}" height="{h}" role="img" aria-labelledby="title desc">\n'
        f'<title id="title">{escape(title)}</title>\n'
        f'<desc id="desc">{escape(desc)}</desc>\n{STYLE}\n{DEFS}\n'
    )


def container(x, y, w, h, kind):
    fill, stroke = COL[kind]
    return (
        f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="14" '
        f'fill="{stroke}" fill-opacity="0.08" stroke="{stroke}" stroke-width="1.5"/>\n'
    )


def box(x, y, w, h, kind, title, sub=None, mono=False, sub_mono=False, size=None):
    fill, stroke = COL[kind]
    cx = x + w / 2
    cls = "b m" if mono else "b"
    if size:
        cls += f'" style="font-size:{size}px'
    out = f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>\n'
    if sub:
        out += f'<text x="{cx}" y="{y + h / 2 - 3}" text-anchor="middle" class="{cls}">{escape(title)}</text>\n'
        scls = "s m" if sub_mono else "s"
        out += f'<text x="{cx}" y="{y + h / 2 + 15}" text-anchor="middle" class="{scls}">{escape(sub)}</text>\n'
    else:
        out += f'<text x="{cx}" y="{y + h / 2 + 5.5}" text-anchor="middle" class="{cls}">{escape(title)}</text>\n'
    return out


def pill(cx, cy, text, mono=True, size=12.5):
    # generous width estimate: 0.62em per char
    w = len(text) * size * 0.62 + 16
    h = size + 10
    cls = "l m" if mono else "l"
    return (
        f'<rect x="{cx - w / 2:.1f}" y="{cy - h / 2:.1f}" width="{w:.1f}" height="{h:.1f}" rx="{h / 2:.1f}" class="pill"/>\n'
        f'<text x="{cx}" y="{cy + size * 0.36:.1f}" text-anchor="middle" class="{cls}">{escape(text)}</text>\n'
    )


def tag(cx, cy, text, kind, size=13, mono=True):
    fill, stroke = COL[kind]
    mc = 'class="m" ' if mono else ""
    w = len(text) * size * 0.62 + 18
    h = size + 12
    return (
        f'<rect x="{cx - w / 2:.1f}" y="{cy - h / 2:.1f}" width="{w:.1f}" height="{h:.1f}" rx="7" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>\n'
        f'<text x="{cx}" y="{cy + size * 0.36:.1f}" text-anchor="middle" {mc}style="font-size:{size}px;font-weight:600">{escape(text)}</text>\n'
    )


def arrow(d, dashed=False, both=False):
    cls = "ln dash" if dashed else "ln"
    start = ' marker-start="url(#a)"' if both else ""
    return f'<path d="{d}" class="{cls}" marker-end="url(#a)"{start}/>\n'


def label(x, y, text, anchor="middle", cls="s"):
    return f'<text x="{x}" y="{y}" text-anchor="{anchor}" class="{cls}" style="fill:{LINE}">{escape(text)}</text>\n'


# ---------------------------------------------------------------------------
# Shell + Core round trip (command_overview.svg and architecture.svg)
# ---------------------------------------------------------------------------

def stack(x, y, w, h, kind, title, sub):
    """A box with two offset cards behind it, the notation for 'many'."""
    fill, stroke = COL[kind]
    out = ""
    for d in (12, 6):
        out += f'<rect x="{x + d}" y="{y + d}" width="{w}" height="{h}" rx="10" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>\n'
    return out + box(x, y, w, h, kind, title, sub)


def many(x, y, anchor="start"):
    """UML-style multiplicity tag."""
    return f'<text x="{x}" y="{y}" text-anchor="{anchor}" class="s m" style="fill:{LINE};font-weight:600">0..n</text>\n'


def round_trip(detail):
    W = 960
    SX, SW = 30, 230  # shell
    BX = 370  # boundary
    CX, CW = 480, 450  # core
    TOP = 60
    L = CX + 30  # core left column
    LW = 200
    MX, MW = 810, 100  # model column
    R1, R2, R3 = 80, 200, 340  # row tops
    RH = 70
    ux, uw = SX + 20, SW - 50   # shell boxes (room for the stack offset)
    HY, HH = 425, 64            # effect handlers (front card)
    BOT = HY + HH + 12 + 14
    H = BOT + (25 if detail else 30)

    if detail:
        title = "Crux: the shell and core round trip"
        desc = (
            "The shell calls process_event with an event. App::update changes the "
            "Model and returns a Command. The Command can send follow-up events back "
            "to update, and it gives zero or more effects to the shell. The shell sends "
            "a Render effect to the UI, which calls view and draws the ViewModel. The "
            "shell sends the other effects to their handlers, and it gives each result "
            "back to the core with resolve."
        )
    else:
        title = "Crux architecture"
        desc = (
            "The shell holds the UI and the effect handlers. The core holds the app's "
            "update and view functions, the Model and the Commands. The shell sends "
            "events to the core. The core gives the shell zero or more effects. The UI "
            "asks the core for the view when it receives a Render effect, and it draws the ViewModel. The effect "
            "handlers run the other effects and return each Output to the core."
        )

    s = head(W, H, title, desc)

    if detail:
        s += f'<text x="{SX + SW / 2}" y="40" text-anchor="middle" class="h">Shell</text>\n'
        s += f'<text x="{CX + CW / 2}" y="40" text-anchor="middle" class="h">Core</text>\n'
        st = f'class="s" style="fill:{LINE};font-weight:600"'
        s += f'<text x="{BX}" y="26" text-anchor="middle" {st}>FFI + generated types</text>\n'
        s += f'<text x="{BX}" y="42" text-anchor="middle" {st}>(for non-Rust shells)</text>\n'
        s += container(SX, TOP - 5, SW, BOT - TOP + 10, "shell")
        s += container(CX, TOP - 5, CW, BOT - TOP + 10, "fn")
        s += f'<path d="M{BX},{TOP - 8} V{BOT + 10}" class="ln dash" style="stroke-width:1.5"/>\n'
    else:
        # Core nested inside the Shell
        s += container(15, 15, W - 30, H - 30, "shell")
        s += f'<text x="35" y="44" class="h">Shell</text>\n'
        s += container(CX, 28, CW, BOT - 28 + 5, "fn")
        s += f'<text x="{CX + 18}" y="57" class="h">Core</text>\n'

    # shell boxes
    # UI box: text sits left of centre, to leave room for the Render -> view path
    rx_ = ux + uw / 2 + 22  # x of the Render path inside the UI
    fill, stroke = COL["shell"]
    s += f'<rect x="{ux}" y="{R1}" width="{uw}" height="{R2 + RH - R1}" rx="10" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>\n'
    tx = (ux + rx_) / 2 - 8
    ty = (R1 + R2 + RH) / 2
    s += f'<text x="{tx}" y="{ty - 10}" text-anchor="middle" class="b">UI</text>\n'
    s += f'<text x="{tx}" y="{ty + 10}" text-anchor="middle" class="s">sends events,</text>\n'
    s += f'<text x="{tx}" y="{ty + 26}" text-anchor="middle" class="s">draws the view</text>\n'
    # Render arrives at the bottom of the UI, and the view request leaves at the top
    s += f'<path d="M{rx_},{R2 + RH - 4} V{R1 + 40} Q{rx_},{R1 + 22} {rx_ + 18},{R1 + 22} H{ux + uw}" class="ln dash" style="stroke-width:1.5"/>\n'
    s += stack(ux, HY, uw, HH, "shell", "Effect handlers", "HTTP, storage, time, …")

    # core boxes
    s += box(L, R1, LW, RH, "fn", "App::view", None, mono=True)
    s += box(L, R2, LW, RH, "fn", "App::update", None, mono=True)
    s += box(L, R3, LW, RH, "cmd", "Command<Effect, Event>", mono=True, size=13.5)
    s += box(MX, R1, MW, R2 + RH - R1, "model", "Model")

    # core internal arrows
    s += arrow(f"M{MX},{R1 + 35} H{L + LW + 2}")
    s += pill((L + LW + MX) / 2, R1 + 35 - 15, "reads", mono=False, size=12)
    s += arrow(f"M{L + LW + 2},{R2 + 35} H{MX - 2}", both=True)
    s += pill((L + LW + MX) / 2, R2 + 35 - 15, "changes", mono=False, size=12)
    # update -> Command (returns) and follow-up Event loop
    gy = (R2 + RH + R3) / 2
    s += arrow(f"M{L + 60},{R2 + RH + 2} V{R3 - 2}")
    s += pill(L + 60 - 44, gy, "returns", mono=False, size=12)
    xu = L + LW - 60
    s += arrow(f"M{xu},{R3 - 2} V{R2 + RH + 2}", dashed=True)
    s += many(xu - 12, R2 + RH + 20, anchor="end")
    s += pill(xu + 76, gy + 8, "follow-up Event", mono=False, size=12)

    # boundary arrows
    a, b = ux + uw + 2, L - 2
    mid = (a + b) / 2
    if detail:
        t_view, t_vm, t_ev = "view() on Render", "ViewModel", "process_event(event)"
        t_eff, t_res = "Vec<Effect>", "resolve(request, result)"
    else:
        t_view, t_vm, t_ev = "on Render", "ViewModel", "Event"
        t_eff, t_res = "Effect", "Output"
    s += arrow(f"M{a},{R1 + 22} H{b}")
    s += pill(mid, R1 + 22 - 14, t_view, mono=detail)
    s += arrow(f"M{b},{R1 + 50} H{a}")
    s += pill(mid, R1 + 50 + 14, t_vm)
    s += arrow(f"M{a},{R2 + 35} H{b}")
    s += pill(mid, R2 + 35 - 14, t_ev)

    # effects: Command -> shell, then fan out to the UI (Render) and the handlers
    jx = ux + uw / 2
    ey = R3 + 22
    fx = jx + 100  # fork point
    s += f'<path d="M{L - 2},{ey} H{fx}" class="ln"/>\n'
    s += many(CX - 8, ey + 18, anchor="end")
    s += pill(mid, ey - 14, t_eff)
    s += arrow(f"M{fx},{ey} C{rx_},{ey} {rx_},{ey} {rx_},{ey - 45} V{R2 + RH + 2}")
    s += pill(rx_ - 48, R2 + RH + 28, "Render")
    s += arrow(f"M{fx},{ey} C{jx},{ey} {jx},{ey} {jx},{ey + 50} V{HY - 2}")

    # handlers -> Command (Output / resolve)
    oy = HY + HH / 2 + 4
    ox = L + 40
    s += arrow(f"M{ux + uw + 14},{oy} H{ox - 60} C{ox},{oy} {ox},{oy} {ox},{oy - 40} V{R3 + RH + 2}")
    s += pill(mid, oy - 14, t_res)

    s += "</svg>\n"
    return s


# ---------------------------------------------------------------------------
# Ports and adapters (crux.svg)
# ---------------------------------------------------------------------------

def ports():
    W, H = 960, 640
    cx, cy = 480, 300
    RO, RC = 245, 122
    RM = 205  # radius of the ring labels, outside the tags
    title = "Crux: the two sides of the shell"
    desc = (
        "The Rust core is in the centre and holds the Model. The shell surrounds the core. Events (blue) cross the "
        "boundary into the core. Effects (red) such as Render, Http, KeyValue and Time "
        "cross the boundary out to the shell. An arrowhead on each tag shows the "
        "direction. On the "
        "driving side, the user works with a UI built in SwiftUI, Jetpack Compose, "
        "React, Leptos, Yew, Dioxus or Ratatui. On the driven side, the shell runs "
        "the effects against APIs, storage and devices on iOS, macOS, Android, the "
        "web, the desktop or the terminal."
    )
    s = head(W, H, title, desc)

    # side labels
    s += f'<text x="{cx - 20}" y="40" text-anchor="end" class="h">Driving (UI)</text>\n'
    s += f'<text x="{cx + 20}" y="40" text-anchor="start" class="h">Driven (I/O)</text>\n'

    # user
    ux = 150
    s += f'<line x1="{ux + 40}" y1="{cy}" x2="{cx - RO}" y2="{cy}" class="ln"/>\n'
    s += f'<circle cx="{ux}" cy="{cy - 38}" r="16" fill="none" stroke="{LINE}" stroke-width="2.5"/>\n'
    s += f'<path d="M{ux - 30},{cy + 30} C{ux - 30},{cy - 15} {ux + 30},{cy - 15} {ux + 30},{cy + 30} Z" fill="none" stroke="{LINE}" stroke-width="2.5"/>\n'
    s += f'<text x="{ux}" y="{cy + 58}" text-anchor="middle" class="h">User</text>\n'

    # external world
    wx = 830
    for i, name in enumerate(["APIs", "Storage", "Devices"]):
        y = cy - 70 + i * 70
        s += f'<path d="M{cx + RO},{cy} C{cx + RO + 50},{cy} {wx - 110},{y} {wx - 60},{y}" class="ln"/>\n'
        s += box(wx - 60, y - 22, 120, 44, "world", name)

    # Shell ring and core circle use the container colours of the other
    # diagrams: a light tint of the kind colour over the page background. The
    # ring is an annulus (even-odd fill), so the core tint sits directly on the
    # page and does not mix with the shell tint. Each outline has the colour
    # of the area that it bounds.
    shell_tint, core_tint = COL["shell"][1], COL["fn"][1]
    s += (
        f'<path d="M{cx - RO},{cy} a{RO},{RO} 0 1,0 {2 * RO},0 a{RO},{RO} 0 1,0 {-2 * RO},0 Z '
        f'M{cx - RC},{cy} a{RC},{RC} 0 1,0 {2 * RC},0 a{RC},{RC} 0 1,0 {-2 * RC},0 Z" '
        f'fill="{shell_tint}" fill-opacity="0.08" fill-rule="evenodd"/>\n'
    )
    s += f'<circle cx="{cx}" cy="{cy}" r="{RO}" fill="none" stroke="{shell_tint}" stroke-width="1.5"/>\n'
    s += f'<circle cx="{cx}" cy="{cy}" r="{RC}" fill="{core_tint}" fill-opacity="0.08" stroke="{core_tint}" stroke-width="1.5"/>\n'
    # divider
    # The divider goes the same short distance out of the ring at the top
    # (between the two side headings) and at the bottom (above the title).
    ext = 25
    s += f'<line x1="{cx}" y1="{cy - RO - ext}" x2="{cx}" y2="{cy - RC}" class="ln"/>\n'
    s += f'<line x1="{cx}" y1="{cy + RC}" x2="{cx}" y2="{cy + RO + ext}" class="ln"/>\n'
    s += f'<text x="{cx}" y="{cy + RO + ext + 42}" text-anchor="middle" class="h" style="font-size:34px">Crux</text>\n'

    def pt(r, deg):
        t = math.radians(deg)
        return cx + r * math.cos(t), cy + r * math.sin(t)

    # Ring labels: horizontal text, centred on the middle of the ring band.
    # Each label is checked so that its box stays inside the band and does not
    # overlap another label. Screen angles: 0 = right, 90 = down.
    FS = 13.5
    LH = FS * 1.15
    placed = []

    def fits(x0, y0, x1, y1):
        # nearest and farthest points of the box from the centre
        nx = min(max(cx, x0), x1)
        ny = min(max(cy, y0), y1)
        near = math.hypot(nx - cx, ny - cy)
        far = max(math.hypot(px - cx, py - cy) for px in (x0, x1) for py in (y0, y1))
        return near >= RC + 4 and far <= RO - 4

    def place(items):
        # Each label sits on a pill, as the arrow labels in the other diagrams
        # do. The checks use the pill box, not the text box.
        out = ""
        for text, deg in items:
            lines = text.split("\n")
            w = max(len(t) for t in lines) * FS * 0.6 + 16
            h = LH * len(lines) + 8
            # Start at the wanted angle on the label radius. If the pill does
            # not fit, try small moves in radius and angle, nearest first.
            for dr, da in sorted(
                ((dr, da) for dr in range(-20, 11, 5) for da in range(-8, 9, 2)),
                key=lambda m: (abs(m[1]), abs(m[0])),
            ):
                x, y = pt(RM + dr, deg + da)
                box_ = (x - w / 2, y - h / 2, x + w / 2, y + h / 2)
                if fits(*box_) and all(
                    box_[2] < b[0] or box_[0] > b[2] or box_[3] < b[1] or box_[1] > b[3] for b in placed
                ):
                    break
            else:
                raise AssertionError((text, deg))
            placed.append(box_)
            out += (
                f'<rect x="{box_[0]:.1f}" y="{box_[1]:.1f}" width="{w:.1f}" height="{h:.1f}" '
                f'rx="{min(h / 2, 11):.1f}" class="pill"/>\n'
            )
            y0 = y - h / 2 + 4 + LH * 0.78
            for i, t in enumerate(lines):
                out += (
                    f'<text x="{x:.1f}" y="{y0 + i * LH:.1f}" text-anchor="middle" '
                    f'style="font-size:{FS}px;font-weight:600">{escape(t)}</text>\n'
                )
        return out

    # Tags sit across the core boundary, because events and effects cross it.
    # A small arrowhead on the leading edge shows the direction: an event
    # points into the core, and an effect points out to the shell.
    core_text = (cx - 62, cy - 36, cx + 62, cy + 40)  # Rust Core and Model

    def clear(b):
        return all(b[2] < o[0] or b[0] > o[2] or b[3] < o[1] or b[1] > o[3] for o in placed)

    def rtag(text, deg, kind, outward, mono=True):
        size = 13
        w = len(text) * size * 0.62 + 18
        h = size + 12
        x, y = pt(RC, deg)
        tb = (x - w / 2, y - h / 2, x + w / 2, y + h / 2)
        # the arrowhead sits on the side edge and points left or right,
        # towards the centre for an event and away from it for an effect
        d = 1 if math.cos(math.radians(deg)) > 0 else -1
        if not outward:
            d = -d
        bx = x + d * (w / 2 + 3)
        pts = [(bx, y - 6), (bx, y + 6), (bx + d * 10, y)]
        cb = (min(p[0] for p in pts), min(p[1] for p in pts), max(p[0] for p in pts), max(p[1] for p in pts))
        pad = 8  # keep a visible gap to the ring labels
        boxes = [(b[0] - pad, b[1] - pad, b[2] + pad, b[3] + pad) for b in (tb, cb)]
        for b in boxes:
            assert clear(b) and clear_of(b, core_text), (text, deg)
        placed.extend(boxes)
        fill, stroke = COL[kind]
        poly = " ".join(f"{a:.1f},{b:.1f}" for a, b in pts)
        return tag(x, y, text, kind, mono=mono) + f'<polygon points="{poly}" fill="{stroke}"/>\n'

    def clear_of(b, o):
        return b[2] < o[0] or b[0] > o[2] or b[3] < o[1] or b[1] > o[3]

    tags = ""
    tags += rtag("Event", 212, "event", False)
    tags += rtag("Render", 148, "effect", True)
    tags += rtag("Http", -58, "effect", True)
    tags += rtag("KeyValue", -22, "effect", True)
    tags += rtag("Time", 14, "effect", True)
    tags += rtag("Your own", 50, "effect", True, mono=False)

    s += place([
        ("SwiftUI", 254), ("Jetpack\nCompose", 230), ("React", 198), ("Leptos", 170),
        ("Yew", 146), ("Dioxus", 125), ("Ratatui", 103),
    ])
    s += place([
        ("iOS", 290), ("macOS", 314), ("Android", 336), ("Web", 12), ("Desktop", 40), ("Terminal", 70),
    ])
    # core contents
    s += f'<text x="{cx}" y="{cy - 10}" text-anchor="middle" class="h" style="font-size:24px">Rust Core</text>\n'
    s += box(cx - 40, cy + 4, 80, 30, "model", "Model")
    s += tags
    # legend
    lx, ly = 30, 480
    for i, (kind, txt) in enumerate([("event", "Event that the core receives"), ("effect", "Effect that the core requests")]):
        fill, stroke = COL[kind]
        y = ly + i * 30
        s += f'<rect x="{lx}" y="{y - 11}" width="26" height="18" rx="5" fill="{fill}" stroke="{stroke}" stroke-width="1.5"/>\n'
        s += f'<text x="{lx + 36}" y="{y + 3}" class="s" style="fill:{LINE};font-size:13px;font-weight:600">{txt}</text>\n'
    s += "</svg>\n"
    return s


with open(f"{OUT}/command_overview.svg", "w") as f:
    f.write(round_trip(True))
with open(f"{OUT}/architecture.svg", "w") as f:
    f.write(round_trip(False))
with open(f"{OUT}/crux.svg", "w") as f:
    f.write(ports())
