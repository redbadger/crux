# Embedded: Rust without std

A Crux core can also run on a microcontroller. The firmware is just another
shell over the same `shared` crate the iOS, Android and web shells use: it turns
input into events, handles the effects the core asks for, and draws the view.
Here we'll look at the counter running on an Adafruit
[Circuit Playground Bluefruit](https://www.adafruit.com/product/4333), a small
board with an nRF52840 (a Cortex-M4F with 1 MB of flash and 256 KB of RAM), ten
RGB LEDs and two buttons. The shell is written with [embassy](https://embassy.dev)
and builds for `thumbv7em-none-eabihf`.

```admonish warning title="Experimental"
Running a core without `std` is a spike, not a supported feature. The
`crux_core`, `crux_http` and `crux_time` changes it relies on are evidence for
an RFC, and their names and shape may change. The findings so far are in the
example's
[SPIKE_NOTES.md](https://github.com/redbadger/crux/tree/master/examples/counter/cpb/SPIKE_NOTES.md).
```

```admonish
This walk-through assumes you have already set up the `shared` crate, as
described in [The core](../../getting_started/core.md).
```

```admonish info
As with the [Leptos](../../part-1/shell/web/leptos.md), [Yew](./yew.md) and
[Ratatui](./ratatui.md) shells, the core and the shell are both Rust, linked
into the same binary, so there is no FFI boundary:
the firmware calls `Core` directly, with native Rust types.
```

## The core without std

The core doesn't change to run on a device. What changes is how `shared` is
built: it has a `std` feature, on by default, and turning it off builds the core
with only `core` and `alloc`.

The top of `shared/src/lib.rs` makes the crate `no_std` when the feature is off.
The core still allocates (commands, effects and the view model all live on the
heap), so it brings in `alloc`, which the firmware provides an allocator for:

```rust,noplayground
// Rust: shared/src/lib.rs
{{#include ../../../../examples/counter/shared/src/lib.rs:nostd}}
```

The feature itself, under `[features]` in `shared/Cargo.toml`, turns `std` back
on in each of the core's dependencies:

```toml
# TOML: shared/Cargo.toml
{{#include ../../../../examples/counter/shared/Cargo.toml:nostd}}
```

For that to work, the dependencies must be declared without their default
features in the first place, which the example does once, in the workspace's
`Cargo.toml`. Every shell that depends on `shared` with its defaults gets `std`
back, so nothing changes for them:

```toml
# TOML: Cargo.toml
{{#include ../../../../examples/counter/Cargo.toml:nostd_deps}}
```

Without `std`, `crux_core` still needs locks for its internal queues. It uses
critical sections for them, through the
[`critical-section`](https://crates.io/crates/critical-section) crate, behind a
feature of the same name. The core doesn't enable it: the firmware does, and
links in an implementation for its chip. `crux_core` refuses to build with
neither `std` nor `critical-section`, and says so. `Core`'s own locks become
try-locks, which panic if the core is used from two execution contexts at once,
so a firmware shell should call it from one task at a time.

Some parts of Crux stay `std`-only. Type generation and the serialising FFI
bridge need `std`. That's one reason the FFI lives in its
own crate, `ffi`, rather than in `shared`: Cargo builds every `crate-type` a
dependency declares, so if `shared` were a `staticlib` the firmware would try
to build one for the microcontroller, and fail. With the FFI split out,
`shared` is a plain Rust library that any Rust shell, including the firmware,
can depend on.

## The firmware shell

The firmware is its own Cargo workspace, in `cpb/`, and the example's workspace
excludes it:

```toml
# TOML: Cargo.toml
{{#include ../../../../examples/counter/Cargo.toml:exclude}}
```

It builds for another target, with its own release profile, but the more
important reason is feature unification. Inside one workspace, Cargo builds
`shared` once with the union of every member's features, so the std shells
would turn `std` back on in the firmware's copy too.

Here's the firmware's manifest. It depends on `shared` with
`default-features = false`, and turns on `crux_core`'s `critical-section`
feature. `cortex-m`'s `critical-section-single-core` provides the
implementation, which is right for a single-core chip. The rest is embassy
for the nRF52840, and an allocator:

```toml
# TOML: cpb/Cargo.toml
{{#include ../../../../examples/counter/cpb/Cargo.toml}}
```

There's no operating system to give us a heap, so the firmware sets one up
from a static array. The counter's core needs very little: every event ends
with one `Render`, so nothing stays in flight between presses.

```rust,noplayground
// Rust: cpb/src/main.rs
{{#include ../../../../examples/counter/cpb/src/main.rs:allocator}}
```

The shell owns the core, along with the hardware it draws on and a little
state of its own:

```rust,noplayground
{{#include ../../../../examples/counter/cpb/src/main.rs:shell}}
```

Sending an event to the core and handling the effects it asks for looks like
it does in any Rust shell. The counter has one effect, `Render`:

```rust,noplayground
{{#include ../../../../examples/counter/cpb/src/main.rs:process_event}}
```

Rendering reads the view model from the core and draws it. The view's `value`
is unbounded, and the board has ten LEDs, so the shell draws it as an odometer:
each lap of ten fills in a new colour over the last.

```rust,noplayground
{{#include ../../../../examples/counter/cpb/src/main.rs:render}}
```

Finally, the main loop. It creates the shell, draws the initial view, and then
waits for whichever input changes first. Button A sends `Increment`, button B
sends `Decrement`, and the slide switch changes the brightness:

```rust,noplayground
{{#include ../../../../examples/counter/cpb/src/main.rs:main_loop}}
```

### Presentation stays in the shell

The brightness and the LED that flashes on each press belong to this board,
so they live in the shell, and the core never hears about them. Changing the
brightness redraws the same view, without an event. The counter's `Reset`
event has no button here, which is fine too: a shell sends the events that
make sense for its UI. Nothing in `shared` is specific to the board, and the
other shells build it exactly as before.

## Build and run

The example's
[README](https://github.com/redbadger/crux/tree/master/examples/counter/cpb)
has the details. Everything goes through `just`:

```sh
cd examples/counter/cpb
just doctor   # check the tools
just build    # release firmware
just flash    # build a UF2 and copy it onto the board
```

To flash the board, plug it in, double-press its reset button so that the
`CPLAYBTBOOT` drive appears, and run `just flash`.

```admonish success
The ring of LEDs shows the count: button A adds a green LED, button B takes
one away, and below zero they turn red.
```

## Capabilities on a device

The counter-http example's core makes HTTP requests with `crux_http`, keeps
the count up to date with Server-Sent Events, and paces reconnecting with
`crux_time`. Its firmware shell runs that core on the same board. Both
capability crates have a `std` feature like `shared`'s, and without it they
keep a subset:

- `crux_http` keeps its protocol types (`HttpRequest`, `HttpResult`),
  `HttpError`, and a `command::Http` builder with a minimal response type. The
  `Client`, middleware and the `http`/`mime` types need `std`.
- `crux_time` keeps its protocol and operation types, `notify_after` and the
  timer handles. Anything that reads the wall clock (`now`, `notify_at`) needs
  `std`.

Its `shared/Cargo.toml` turns `std` on in each of them:

```toml
# TOML: shared/Cargo.toml
{{#include ../../../../examples/counter-http/shared/Cargo.toml:nostd}}
```

The board has no IP stack, so the shell sends the core's HTTP requests and
event streams over Bluetooth LE to a gateway: a page in Chrome, itself a Crux
app, which performs them with `fetch` and sends back the answers. The shell
handles each effect as any other shell does, with the link standing in for
the network:

```rust,noplayground
// Rust: cpb/src/main.rs (counter-http)
{{#include ../../../../examples/counter-http/cpb/src/main.rs:handle}}
```

When the gateway answers, the shell resolves the request it kept, and handles
the effects that follow:

```rust,noplayground
{{#include ../../../../examples/counter-http/cpb/src/main.rs:http_answer}}
```

The timers run in an embassy task of their own. It holds a `&'static Core`,
resolves each timer when it fires (as `crux_time`'s shipped shell handlers
would), and queues the effects that follow for the main loop, so only one task
is in the core at a time:

```rust,noplayground
// Rust: cpb/src/delay.rs (counter-http)
{{#include ../../../../examples/counter-http/cpb/src/delay.rs:fire}}
```

The
[counter-http firmware's README](https://github.com/redbadger/crux/tree/master/examples/counter-http/cpb)
explains how to run it end to end, and what the board shows. The BLE protocol
and the gateway don't know anything about the counter, and live in
[`examples_support/ble_protocol`](https://github.com/redbadger/crux/tree/master/examples_support/ble_protocol)
and
[`examples_support/ble_gateway`](https://github.com/redbadger/crux/tree/master/examples_support/ble_gateway).
