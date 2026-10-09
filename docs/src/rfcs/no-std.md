# RFC: `no_std` + `alloc` support

```admonish
This RFC is **proposed**. It contains no implementation code. The evidence in
it comes from a throwaway spike, which is not part of this pull request, that
built `crux_core` for a Cortex-M4 and linked a small Crux app into firmware for
the Adafruit Circuit Playground Bluefruit. That firmware has since been flashed
to the board and works as intended, first with a `spin` lock and then with the
`critical-section` design this RFC recommends. Heap use was measured on a host,
not on the device.

Since then the spike has made the counter and `counter_http` examples' own
cores build without `std` and rebuilt both firmwares as shells over them, and
both have been flashed and work. Work the spike has planned but not done is
marked as planned.
```

This RFC proposes that `crux_core`, and the code its macros emit, build without
the standard library on targets that have a heap and atomic compare-and-swap.
Everything that needs `std` today would sit behind a new `std` feature, which is
on by default, so existing apps would build exactly as they do now.

## Summary

`crux_core` would become `#![no_std]` when its `std` feature is off, and depend
on `alloc` in all builds. The work falls into a handful of blockers, each small:

- swap `std::` paths for `core::` and `alloc::`, with an internal prelude for
  `Vec`, `Box` and `String`;
- replace `crossbeam-channel` and `futures::channel::mpsc`, neither of which
  has a `no_std` mode, with one small internal channel, used in std builds too;
- put `Mutex` and `RwLock` behind an internal shim that is `std::sync` under
  `std` and an embedded-friendly lock otherwise;
- move the serialising FFI bridge behind a `bridge` feature, which requires
  `std`;
- adjust the effect middleware and the `#[effect]` macro output so neither
  names anything that is missing without `std`.

This is `no_std` with `alloc`, not a heap-free Crux. `Command`, the executor
and the effect registries all allocate, and removing that is out of scope.

Beyond `crux_core`, capability crates would offer a `no_std` subset behind the
same kind of `std` feature, and an example's `shared` crate would build without
`std` so that firmware is one more shell over the same core.

## Why?

A Crux core is a pure function of events and a model, with every side effect
described as data and handed to a shell. Nothing about that needs an operating
system. Firmware is a natural shell for it: the shell reads buttons and
sensors, drives LEDs and radios, and resolves the requests the core makes, and
the core holds the behaviour that is worth testing on a host.

On a microcontroller the shell is written in Rust and calls `Core` directly, as
the Leptos and Yew shells already do. There is no FFI and no serialisation, so
the parts of Crux that genuinely need `std` (bincode over `std::io`, the
bridge, type generation, the test harness) are exactly the parts such a shell
does not use. What stops Crux building today is the rest: paths spelled
`std::` that could be `core::` or `alloc::`, two channel crates without a
`no_std` mode, `std::sync` locks, a `ThreadId` in a middleware guard and two
`eprintln!` calls.

The motivating case is the Circuit Playground Bluefruit, an nRF52840 board
with two buttons, a slide switch, a red LED and ten NeoPixels. The spike's
first counter app on it used a render effect, a custom async `Delay` operation
resolved by the shell, and a `Command::new` task that awaited it and sent a
follow-up event. A second firmware runs the `counter_http` app, with HTTP
and server-sent events reaching the network over Bluetooth. That is
a realistic slice of Crux, and the spike shows it fits comfortably in the
chip's 1 MB of flash and 256 KB of RAM.

The aim is one core for every shell: the same `shared` crate that the iOS,
Android and web shells use, built without `std` for the firmware, with the
device's own presentation (pixels, brightness, LEDs) in the firmware shell.
The spike has done that for the counter and `counter_http` examples.

## Goals

- `crux_core` builds and works with `--no-default-features` on Cortex-M
  targets that have compare-and-swap, with `thumbv7em-none-eabihf` as the
  first one kept green in CI.
- It is correct on multi-core chips, such as the RP2040 or a dual-core ESP32,
  as long as one execution context owns the `Core`. Wakers may fire on any
  core and in interrupt handlers.
- Nothing changes for apps that use the default features. Their behaviour,
  public API and dependency on `std` stay as they are.
- Code emitted by `#[effect]` and `#[derive(Operation)]` compiles in a
  `#![no_std]` crate.

## Non-goals

- Targets without compare-and-swap, such as `thumbv6m-none-eabi` (Cortex-M0
  and M0+). They are blocked outside Crux as well as inside it; see
  [Evidence](#evidence).
- The FFI bridge without `std`. A firmware shell calls `Core` directly. A
  non-Rust shell over FFI without `std`, for example C firmware, would need a
  `no_std` wire format, which is left as an open question.
- The full API of the capability crates without `std`. Each can offer a
  `no_std` subset behind a default `std` feature, as described in
  [Capability crates](#8-capability-crates), but making one API serve both
  modes is left to their own RFCs. `crux_kv` is not touched.
- Running without a heap.

## Design

Sections 1 to 7 are each one blocker in `crux_core`, with the fix the spike
used and what it costs. Sections 8 and 9 cover the capability crates and the
examples.

### 1. Prelude and paths

With `std` off, the crate root gains

```rust
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;
```

On the host that alone produces 142 errors, all at the name-resolution stage
(so a floor, not the full count). Most are `Vec` and `Box` no longer being in
scope, and the rest are `use std::...` lines and a few inline `std::` paths,
plus `vec!` in the bridge and the two `eprintln!` calls.

The fix is mechanical. A crate-private `prelude` module re-exports `Box`,
`Vec`, `String`, `ToString` and `ToOwned` from `alloc`, and each module that
needs it imports it. `std::{fmt, mem, any, future, pin, task, ops, marker}`
and `std::sync::atomic` become `core::`, while `std::sync::{Arc, Weak}`,
`std::collections::VecDeque` and `std::task::Wake` become `alloc::`. Modules
that stay std-only (`testing`, `bindgen`, `type_generation` and the bridge)
can keep their `std::` paths.

To stop this regressing, the clippy lints `std_instead_of_core`,
`std_instead_of_alloc` and `alloc_instead_of_core` can be turned on for
`crux_core`.

### 2. An internal channel

`Command` and its executor are built on `crossbeam-channel` (in
`command/mod.rs`, `executor.rs`, `context.rs` and `stream.rs`), and
`ShellStream` uses `futures::channel::mpsc`. `crossbeam-channel` has no
`no_std` mode, and `futures::channel::mpsc` is behind `futures`' `std` feature.
Everything else Crux uses from `futures` (`BoxFuture`, `AtomicWaker`,
`FuturesUnordered`, `Stream`, `Sink`, `StreamExt`) works with `alloc` alone.

The spike replaced both with one unbounded multi-producer, single-consumer
channel of about 150 lines: an `Arc` around a locked `VecDeque`, a sender
count, a flag saying whether the receiver is alive, and an `AtomicWaker` for
the async side. It is used in std builds as well, so the existing test suite
exercises it, and `crossbeam-channel` drops out of `crux_core`'s dependencies
(tests that use it directly keep it as a dev-dependency).

The executor depends on the channels' disconnect behaviour, and the
replacement has to keep it exactly:

- `send` fails once the receiver has been dropped. `JoinHandle::poll` reads
  that as "the task is gone".
- Dropping the receiver drops everything still queued. That releases wakers
  and resolve callbacks, which is what the `Arc::strong_count(&arc_waker) < 2`
  cancellation check in the executor relies on.
- Dropping the last sender wakes the async receiver, which then returns
  `None` without registering its waker again.

The lock is held only around the `VecDeque` operations, and the destructors of
drained items run outside it, because they can touch other channels.

`ShellStream` is public and its variants carry the receiver, so the receiver
type becomes part of the public API. The spike made the internal `sync` module
`#[doc(hidden)] pub` to allow that. A cleaner version would make `ShellStream`
opaque. Either way it is a small change to a type that apps do not normally
name.

The cost is that crossbeam's lock-free queue is replaced by a lock taken on
every send and receive. That has not been measured on the multi-threaded
middleware paths, and it should be before this goes in.

### 3. A sync shim, and the choice of lock

`Core` holds its model in a `std::sync::RwLock` and its root command in a
`std::sync::Mutex`, and the bridge registry, the effect registry and the
buffer route each hold a `Mutex`. All of them call `.expect("... poisoned")`
on every lock.

The spike first added a crate-private `sync` module with `Mutex` and `RwLock`
types. Under `std` they were thin wrappers over `std::sync` whose `lock()`,
`read()` and `write()` return the guard and panic on poison, as every call site
already did. Without `std` they were re-exports of `spin::Mutex` and
`spin::RwLock`, which have the same method names and no poisoning. Call sites
lost their `.expect`, and `crux_core` kept `unsafe_code = "forbid"`.

That first shape compiled, linked and ran, but the real question is which lock
to use without `std`. There are two candidates.

**`spin`** keeps the shim trivial, because its guards look like
`std::sync`'s. It busy-waits, which on a single core means a lock that is
already held is never freed if the waiter interrupted the holder. So `spin`
is only correct if nothing that takes a Crux lock ever runs in an interrupt
handler.

**`critical-section`** is the lock most embedded Rust uses, and the one
embassy is built on. Its `Mutex<T>` hands out `&T` only for the lifetime of a
`CriticalSection` token, so the usual pattern is a
`critical_section::Mutex<RefCell<T>>`, read through `borrow_ref(cs)` or
`borrow_ref_mut(cs)` inside `critical_section::with(|cs| ...)`. There is no
guard type that outlives the closure. The crate defines only the interface;
the firmware links one implementation, for example the
`critical-section-single-core` feature of `cortex-m` (which the spike's
firmware enables for embassy already) or the one a HAL such as `esp-hal`
provides. Forgetting it is a link error, not a silent fallback.

That closure style matters because of one detail in the executor. The waker
the executor gives each task, `CommandWaker`, sends the task's id on the ready
channel in `wake_by_ref`, which takes the channel's lock. A `Command::new` task
can await any future, so if one awaits a future woken from an interrupt (an
embassy timer, or a GPIO edge), that `send` runs in the interrupt handler. If
thread mode happens to hold the same channel lock at that moment, a spinlock
deadlocks the chip. With `critical-section` it cannot happen, because thread
mode holds the lock with interrupts masked.

**This RFC recommends `critical-section` without `std`, and `std::sync` with
it**, for two reasons:

1. it is safe when wakers fire from interrupt handlers, which the executor's
   design makes likely as soon as a task awaits a hardware future;
2. it is what the embedded ecosystem, embassy included, already expects, so a
   firmware app already has an implementation linked.

It has costs, and they shape the shim:

- The shim becomes closure-based (`with(|value| ...)`) for the locks it
  covers, rather than returning a guard.
- A critical section masks interrupts for as long as it lasts. That is fine
  for the channel and the registries, which hold their locks for a few
  instructions, but not for `Core`'s own two locks: `process_event` holds the
  model lock across `App::update`, and `process` holds the root command lock
  across the whole loop of updates and task polls. Wrapping those in a
  critical section would mask interrupts for the entire update.
- On a multi-core chip, the critical-section implementation adds a hardware
  spinlock to the masking. It is one lock for the whole chip, so while Crux
  holds it the other core's critical sections wait, including its allocator's
  and embassy's channels. That is correct, but it is another reason to keep
  the closures short.

So the split is `critical-section` for the short-held internal locks, and,
for `Core`'s model and root command, a non-blocking lock that panics on
contention without `std`. That turns "a `Core` is used from one execution
context at a time" into a checked rule, which a firmware shell like the spike's
satisfies naturally.

The spike then tried this split:

- **Internal locks.** `sync::Mutex<T>` has one method, `with(|value| ...)`.
  Under `std` it wraps `std::sync::Mutex`. Without it, it is a
  `critical_section::Mutex<RefCell<T>>` inside `critical_section::with`, so
  re-entering the same lock panics on the `RefCell` borrow instead of
  deadlocking. Eleven call sites in the channel, the effect registry, the
  buffer route and the bridge registry became closures. None was awkward. Two
  rules kept the critical sections short:
  - `EffectRegistry::resolve` resolves the request (which wakes tasks, and so
    takes the channel's lock) outside the lock, and the channel wakes its
    receiver outside it.
  - `Buffer::drain` takes the whole vector instead of collecting a new one.

  A `VecDeque` or slab that grows still allocates inside a critical section.
- **`Core`'s locks.** `CoreMutex` and `CoreRwLock` keep the guard API, so
  `Core`'s code barely changes. Under `std` they block, as today. Without it
  they wrap the [`try-lock`](https://crates.io/crates/try-lock) crate's
  `TryLock`, a small `no_std` crate with one atomic flag and no waiting.
  Contention panics with a message naming the one-context rule. `read()` is
  exclusive too, which loses nothing when one context owns the `Core`. The
  crate supplies the `UnsafeCell` that `unsafe_code = "forbid"` stops
  `crux_core` writing itself.
- **Rejected: moving the model out and back.** Keeping `Core`'s values in a
  `critical_section::Mutex<RefCell<Option<T>>>`, taking them out for each call
  and putting them back, would avoid the second crate. It also moves the model
  on the stack on every call, which is a real risk on a small chip.

`spin` goes away entirely. Both new crates are optional, behind a
`critical-section` feature, and a `compile_error!` fires when neither it nor
`std` is enabled. What the spike measured is in [Evidence](#evidence).

`try-lock` needs compare-and-swap, as `spin` did, so this does not help chips
without it. Those are a non-goal.

### 4. A `bridge` feature

`bincode` 1.3 is built on `std::io`, and the bridge also uses `serde_json` and
`HashMap`. Rather than port them, a new default feature, `bridge`, would gate
`mod bridge`, the `EffectFFI` trait and its re-export, `middleware::Bridge`
with `Layer::bridge` and the format re-exports, and the `Serialized` route in
`effects::routes`, which is built on the bridge's resolve registry.

`bridge` requires `std`. `facet_typegen` requires `bridge`, because the
generated code describes the bridge's wire types and the `Export` impl from
`#[effect(facet_typegen)]` names `crux_core::bridge::Request`. Keeping
`bridge` separate from `std`, rather than implied by it, lets an app ask for
`std` without the bridge. In the examples, only the FFI crate turns it on (see
[Examples](#9-examples)).

### 5. Middleware without `std`

The reentrancy guard in `middleware/effect_handling.rs` panics if
`try_process_effect` resolves a request synchronously on the same thread
(issue #492). It records a `ThreadId`, which does not exist without `std`.
Without it the guard is the `AtomicBool` alone, so any `resolve()` while
`try_process_effect` is on the stack panics, including one from another core
or an interrupt handler. That is stricter than the std behaviour, but
consistent with it.

The two `eprintln!` calls would go through `log`, which is already an
optional dependency, so a firmware app can route them to `defmt` or drop
them. The spike simply dropped them.

The alternative is to gate effect middleware on `std`. Middleware exists to
let the core hand effects to Rust code in the same process, which is less
likely to be wanted on a microcontroller, but nothing in it needs `std` apart
from these two points.

The effect router in `crux_core::effects`, the other way to handle effects in
Rust, needs nothing beyond the path and lock changes above. Only its
`Serialized` lane needs `std`, and section 4 puts that behind `bridge`.
`EffectRouter`, `ResolveSink` and the `Parked` and `Buffer` lanes build for
`thumbv7em-none-eabihf` with the rest of the spike's `crux_core`.

### 6. Macro output

Most macro output already works in a `#![no_std]` crate. The spike tested the
macros against the firmware crate (edition 2024, `#![no_std]`):

- `#[derive(Operation)]` and `#[operation(request, output = ())]` needed no
  changes, because their output uses `::core::option::Option` and
  `::crux_core` paths.
- Plain `#[effect]` worked unchanged. Its `From` and `TryFrom` impls and its
  `is_*` and `into_*` helpers use only names from the core prelude, and its
  testing items are already behind `__crux_core_testing_items!`.
- `#[derive(Facet)]` compiled with `facet` at `default-features = false,
  features = ["alloc"]`.

`#[effect(facet_typegen)]` did not. It emits an `<Effect>Ffi` enum with serde
attributes and an `EffectFFI` impl, which gave five errors. The spike added a
`__crux_core_bridge_items!` macro to `crux_core`, modelled on
`__crux_core_testing_items!`, which expands to its input only when `crux_core`
has `bridge`, and wrapped those items in it.

That works, but has two costs. The FFI enum is then silently not generated
when `bridge` is off, which may surprise someone. And `prettyplease` prints the
tokens inside a macro invocation verbatim, so the macro snapshot tests lose
their formatting; the spike had to compare
`the_largest_effect_enum_the_id_can_describe` with whitespace stripped. The
alternative is a hard error, emitted by the same kind of gate, that says
`facet_typegen` needs `crux_core`'s `bridge` feature. That is clearer for the
user and leaves the snapshots readable.

The examples now argue the other way. The counter's `shared` crate keeps
`#[effect(facet_typegen)]` and builds without `std`, and also with `std` but
without `bridge` when a Rust shell is built on its own, because only the FFI
crate enables `bridge` (section 9). With a hard error, every such core would need
`cfg_attr` around the attribute. Open question 4 asks for a decision.

### 7. Feature layout

```toml
[features]
default = ["std", "bridge", "crux_macros"]
std = ["facet/std", "futures/std", "serde/std", "slab/std", "thiserror/std"]
critical-section = ["dep:critical-section", "dep:try-lock"]  # the locks without std
bridge = ["std", "dep:bincode", "dep:serde_json"]
testing = ["std", "dep:anyhow"]
facet_typegen = ["bridge", "crux_macros/facet_typegen", "dep:facet_generate", "dep:heck", "dep:log"]
uniffi_compat_bindgen = ["std", "dep:anyhow", "dep:camino", "dep:cargo_metadata", "dep:uniffi_bindgen"]
```

`futures`, `serde`, `facet`, `slab` and `thiserror` move to
`default-features = false` (with `alloc` where they have it), and their std
features come back through `std`. `anyhow`, `bincode` and `serde_json` become
optional. The spike checked that every one of `std`, `std,crux_macros`,
`crux_macros`, `testing`, `uniffi_compat_bindgen`, `facet_typegen` and no
features at all builds on the host.

The differences an app would see without `std`:

| | `std` | no `std` |
|---|---|---|
| Internal locks | `std::sync`, panic on poison | `critical-section`, masks interrupts briefly |
| `Core`'s model and root command | `std::sync`, blocks | try-lock, panics on contention |
| Middleware reentrancy guard | panics on a same-thread `resolve` during `try_process_effect` | panics on any `resolve` during it |
| Middleware diagnostics | `eprintln!` today, `log` proposed | `log` proposed (silent in the spike) |
| `bridge`, `EffectFFI`, `Serialized` route, `middleware::Bridge` | available | absent |
| `testing`, `facet_typegen`, `uniffi_compat_bindgen` | available | require `std` |
| `#[effect(facet_typegen)]` | FFI enum and `EffectFFI` impl | FFI items omitted (open question 4) |

`serde` is still compiled into firmware, because `crux_core` derives
`Serialize` and `Deserialize` on `RenderOperation` and similar types
unconditionally. It costs about 270 bytes in the spike's firmware, and could
become optional along with the bridge later.

### 8. Capability crates

A core that uses a capability crate needs that crate without `std` too. The
pattern the spike settled on is a `no_std` subset: a default `std` feature
carries the full API, and without it the crate keeps what a firmware shell can
serve.

- **`crux_http` (done in the spike).** `http` and `mime` have no `no_std`
  mode and are in its public API, so without `std` the crate keeps the
  protocol types, `HttpError` and `expect`, and swaps in twins of
  `crux_http::Response<T>` and `command::{Http, RequestBuilder}` from
  `src/nostd/`. The twins have the same names and call shapes, not the same
  types: `status()` is a `u16`, headers are a `Vec<HttpHeader>`, and
  `Http::request` takes the method as a string. `counter_http`'s HTTP code
  compiles unchanged against either. `Client`, middleware, `.query()` and
  `testing` stay std-only. The serialised types are identical, so a `no_std`
  device and a std gateway agree on the wire. The subset builds for
  `thumbv7em-none-eabihf`, has its own seven tests, and runs on the board
  (see [Evidence](#evidence)). One fix is needed outside it:
  `facet-generate-attrs` must depend on `facet` with `default-features =
  false`, or it turns `facet/std` on for the whole build.
- **`crux_time` (done in the spike).** A default `std` feature and
  `core::time::Duration`. The protocol types, `clock::Time::notify_after`,
  the timer handles and the `NotifyAfter` and `ClearTimer` operations work
  without `std`. `clock::Time::{now, notify_at}`, which name `SystemTime`, and
  the deprecated root `Time` stay std-only. It has its own `no_std` test, and
  `just check-nostd` and CI check it, with `crux_http`'s subset, for
  `thumbv7em-none-eabihf`.
- **`crux_kv`** is not touched.

This is how cores should wait for time: through `crux_time`'s `NotifyAfter`
and `ClearTimer`, as effect variants `TimeNotifyAfter` and `TimeClear`, rather
than an app's own `Delay` operation as in the spike's first firmwares. The
shell serves them the same way it served `Delay`, and the `counter_http`
firmware now does.

### 9. Examples

The spike's first firmware carried its own copy of each app. It now builds the
counter example's own core instead, and the layout that took is worth
keeping:

- **`shared` is the app.** It is a plain `lib` with a default `std` feature
  (`crux_core/std`, `facet/std`, `serde/std`), and builds without `std` when
  that is off. The firmware depends on it with `default-features = false` and
  enables `crux_core/critical-section` itself.
- **The FFI moves to a sibling `ffi` crate** (package `shared_ffi`), with
  `CoreFfi`, BoltFFI, `boltffi.toml`, the `codegen` binary and the `cdylib`
  and `staticlib` crate types. Cargo builds every declared crate type of a
  dependency, so a `staticlib` on `shared` fails for a `thumbv7em` dependent,
  which has no panic handler or allocator. BoltFFI reads the crate types from
  `cargo metadata`, so they must stay declared, on the FFI crate. That crate
  is also the one that enables `crux_core/bridge`; the Rust shells depend on
  `shared` alone. The native shells' module and package names are unchanged;
  only the native library files are now named after `shared_ffi`.
- **The firmware is its own workspace**, beside the other shells
  (`examples/counter/cpb`) and listed in the example workspace's `exclude`,
  so feature unification with the std shells cannot pull `std` into it.
- **Workspace dependencies.** The example's `[workspace.dependencies]` entries
  for `crux_core`, `facet` and `serde` have `default-features = false`, and
  `shared`'s `std` feature turns them back on.

The counter's view model gained a numeric `value` beside its display string,
so the firmware can draw the count. Brightness and the LED flash are
presentation, kept in the firmware shell, so the counter's core needs no timer
at all.

`counter_http` now has the same split, and its firmware is a shell at
`examples/counter-http/cpb`. Its core adopted what the spike's copy had added
for a board with nobody to reload a page, so every shell benefits: an error
state in the model and view in place of a `panic!`, and reopening the
server-sent-events stream when it ends, after a `crux_time` back-off of 1 s
doubling to 30 s. For that, every shell ends a failed stream with `Done`. The
firmware answers `NotifyAfter` and `ClearTimer` from an embassy task, and its
`Delay` is gone. Link-up and link-down stay in the shell: it sends the same
start-up events as the other shells, and on link-down fails the requests in
flight and ends the streams. The app-agnostic Bluetooth protocol and the
browser gateway are in `examples_support/`: `ble_protocol` is a member of the
root workspace, with its own wire copy of the SSE types, and `ble_gateway` is a
workspace of its own.

## Drawbacks

**Workspace dependencies have to change.** `facet.workspace = true` and
`serde.workspace = true` cannot be used by a `no_std` `crux_core`, because the
workspace entries enable the crates' default features and a member cannot turn
them off. The spike spelled out versions in `crux_core`. The proper fix is
`default-features = false` in `[workspace.dependencies]`, with every member
that needs `std` opting back in, which touches every crate in the repository.
Each example workspace needs the same, as section 9 describes.

**The repository root has to exclude the examples.** The root `Cargo.toml`
needs `exclude = ["examples"]`. When Cargo looks for a crate's workspace root,
it first checks the roots it has already loaded, for every ancestor
directory. The firmware depends on `crux_core` by path, which loads the
repository root, so without the exclude a path dependency on an example's
`shared` resolves its `workspace = true` keys against the repository root
rather than the example's.

**Each example gains a crate.** Splitting the FFI out of `shared` adds an
`ffi` crate to every example that has native shells, and renames their native
library files.

**A feature that every `no_std` crate has to name.** Cargo cannot express
"this dependency only when a feature is off", which is why the spike's first
lock, `spin`, was unconditional and sat in every app's lockfile. The
`critical-section` feature avoids that, and a plain std app's lockfile no
longer lists any lock crate. It has its own costs, which the spike found:

- A `no_std` crate that depends on `crux_core` fails the `compile_error!`
  when built on its own unless something enables the feature, including
  library crates that never link a binary. The spike enables it in the
  firmware, and on the command line when checking a library such as the
  counter's `shared` for the target, so libraries need no forwarding feature.
- Feature unification brings the two crates into std builds that share a
  graph with such a library. In the spike, a std-only gateway that depends on
  the `no_std` protocol crate compiles them, harmlessly, because the `std`
  locks win.
- A `no_std` build that runs on a host, such as a heap probe, must link
  `critical-section`'s `std` implementation. Forgetting it is a link error.

**A second build mode to keep green.** Without a CI job that builds
`crux_core` and a small example for `thumbv7em-none-eabihf` on every pull
request, the `no_std` path will break quietly within weeks.

**Macro snapshots get harder to read**, if the bridge gate wraps the
generated items in a `macro_rules!` invocation (design section 6).

**The internal channel is new code on a hot path**, replacing a well-tested
crate, with its performance under contention not yet measured.

**One core means the device carries the whole app.** `counter_http`'s view
formats a `chrono` timestamp into `text`, which the board never reads, and
that is most of the 9 KB its firmware grew by (see [Evidence](#evidence)).
Keeping link state out of the core also has a small cost: requests failed on
link-down show the core's error until the next update after reconnecting.

**A capability crate's subset is a second API to keep in step.** Every method
added to `crux_http`'s std builder needs a decision for the `no_std` twin.

## Migration

Apps on the default features notice nothing. Their lockfiles lose nothing and
gain nothing.

Apps that turn default features off today would need to add `std` (and
`bridge`, if they use it) to keep what they have.

While surveying this, the spike found a pre-existing bug that affects std
users too. `crux_core/src/lib.rs` re-exports the macros under

```rust
#[cfg(feature = "default")]
pub use crux_macros as macros;
```

so `crux_core::macros` disappears whenever default features are off, even if
`crux_macros` is enabled explicitly. The gate should be
`#[cfg(feature = "crux_macros")]`. That fix is independent of this RFC and is a
candidate for a small standalone pull request.

## Alternatives considered

**Keep `std`, and run Crux on an RTOS that provides it.** ESP-IDF, for
example, gives the ESP32 family a `std` port, and Crux would probably run
there today. It does nothing for the much larger set of chips with no `std`
port at all, including the nRF52 and RP2040 families, and it brings an RTOS
into firmware that may not otherwise want one.

**A separate `no_std` core crate, or a fork.** This would leave `crux_core`
untouched, but two implementations of `Command` and the executor would drift
apart, and the macros would have to target both. The changes needed are small
enough to make one crate with a feature the better trade.

**Gate more, port less.** Gating `Command` itself on `std` and offering a
minimal synchronous core without `std` would avoid the channel and lock work.
It would also lose async tasks, which are the reason to use Crux rather than a
plain state machine.

## Evidence

The spike made `crux_core` and the macro output build for
`thumbv7em-none-eabihf`, then linked a counter app of its own into
`embassy-nrf` firmware for the Circuit Playground Bluefruit. It was built on a branch close to master;
every type, path and feature named in this RFC was checked against master.

**Errors.** With only the `std` feature and `#![no_std]` added, the host build
gives 142 errors at name resolution. The target build fails earlier, in
`futures` and `memchr`, because `crux_core` depends on `futures` with its
default features. Once dependency features are fixed, `crossbeam-utils` fails
next; once `crossbeam-channel`, `bincode` and `serde_json` are made optional,
`crux_core` itself is reached with 162 errors. Every hard blocker in that list
had been predicted by reading the source.

**Tests.** With default features, all 483 workspace tests pass after the
`#[effect]` snapshots are updated, and the counter example builds and
passes its tests.

**Size.** The release firmware is 21,652 bytes of text, 24 bytes of data and
34,012 bytes of bss, of which 32 KB is the heap. `crux_core` accounts for
5.0 KiB of flash; with the `futures`, `slab` and `serde` code it pulls in,
Crux costs roughly 6 to 7 KiB for this app.

**Heap.** The app was run unchanged on a 32-bit host target with a counting
allocator, driven the way the firmware's shell drives it. Peak live heap was
1,956 bytes for one button press with its flash in flight, 8,644 bytes for
five overlapping presses and 29,784 bytes for twenty, about 1.5 KB each. Idle
heap is 568 bytes, rising to 2,600 bytes after the burst of twenty as slab and
queue capacity is retained; that is stable across repeats, not a leak. Heap
grows with the number of concurrent commands, because each `Command` allocates
several small things: four channels, an `Arc` per task flag and a boxed
future. Allocator overhead is not included, and fragmentation over a long run
has not been tested.

**No compare-and-swap.** `thumbv6m-none-eabi` fails before reaching
`crux_core`. `spin` gives 13 errors, because `compare_exchange` and the
`fetch_*` methods do not exist on that target's atomics. Switching on `spin`'s
`portable_atomic` feature moves the failure to `portable-atomic`, which then
needs a `critical-section` implementation, and to `facet-core`, which uses
`alloc::sync::Arc` unconditionally. Beyond that, `alloc::sync` does not exist
without pointer-sized atomics, `crux_core` uses `Arc` throughout, and
`futures-util` gates `AtomicWaker` and `FuturesUnordered` on them. Supporting
these chips would need `portable-atomic` and its `Arc` (which cannot be a
`self: Arc<Self>` receiver for `Wake`), `futures`' `portable-atomic` feature,
a lock that does not need compare-and-swap, and a change upstream in facet.

**On hardware.** That first firmware, with its own counter app and a `Delay`
effect, was flashed through the board's UF2 bootloader and runs as intended.
Button and switch interrupts reach the `embassy` executor, and each press
updates the model and redraws the NeoPixels. The
async command that awaits the shell's `Delay` request and then sends a
follow-up event also completes, so the internal channel, the command executor
and request resolution all work on the Cortex-M4 with the spike's `spin`-based
lock. The only fault was contact bounce in the spike's own shell, which was
fixed there; `crux_core` needed no change. This was short, manual testing:
heap use, timing and long-running behaviour were not measured on the device.

**The recommended lock.** With `spin` replaced by the design in section 3, all
of `crux_core`'s tests still pass, and the `compile_error!` fires on a
`no_std` build without the feature. Two firmware images were measured before
and after on the same day:

| | `spin` | `critical-section` and try-lock |
|---|---|---|
| Counter firmware text | 21,356 B | 20,728 B (−628 B) |
| HTTP-over-BLE firmware text | 327,288 B | 326,432 B (−856 B) |
| HTTP app peak heap, 32-bit host | 45,435 B | 45,435 B |

The second image is a larger spike firmware that runs a copy of the
`counter_http` example's app, built on `crux_http`'s `no_std` subset (design
section 8), and reaches the network through a Web Bluetooth page. Its HTTP code
is the example's, unchanged; the copy differs in its view, events for the link
and the switch, an error state and the stream back-off. Data and
bss did not change, and neither did heap use. On a 64-bit host the counter
probe shows 64 bytes more, which is one-time setup in `critical-section`'s
`std` implementation, not Crux.

Flash shrinks, probably because a single-core critical section on a Cortex-M
is a few instructions to save and restore the interrupt mask, against `spin`'s
compare-and-swap loops. That cause was not confirmed against a same-day `spin`
build.

The HTTP firmware was flashed and run against the live server:
- a GET and a server-sent-events stream;
- button presses sending POSTs;
- the stream being dropped and reopened by the board four times in a row.

All of it worked, with no contention panic.

**A wake from an interrupt.** Neither firmware above wakes a `Command` task
from an interrupt, because each awaits a shell request. To exercise that
path, the counter's flash was changed temporarily to await an `embassy_time`
`Timer` inside the `Command`. This was a throwaway experiment, not part of the
spike's code. Two things outside the lock were needed to make it run at all:

- **Telling the shell about the wake.** Crux's executor only runs when the
  shell calls `Core`, and a timer wake is not an effect, so nothing tells the
  shell to call it. The experiment borrowed `Core::with_waker` and a public
  `Core::process` from an unmerged branch: the shell's waker raised an
  embassy signal, and its main loop called `process()`. This RFC does not
  propose exposing a root waker, because making the timer an effect, such
  as `crux_time`'s `NotifyAfter`, is enough; see below and open question 9.
- **A different embassy timer queue.** embassy's default timer queue rejects
  any waker its own executor did not create, and Crux's `CommandWaker` is not
  one of those, so `Timer` panicked inside a `Command`. The `generic-queue-N`
  feature of `embassy-time` stores any waker. It cost 520 bytes of data for
  32 slots.

With both in place, the whole wake chain runs in the RTC interrupt handler:

1. The timer wakes the task's `CommandWaker`.
2. The waker sends on the child command's ready channel.
3. That wakes the root command's `CommandWaker`, which sends on the root's
   ready channel.
4. That wakes the shell's waker.

With `spin`, steps 2 and 3 are where the chip deadlocks if thread mode holds
the same channel lock at that moment. On the board, every press blinked the
LED and turned it off again, even under sustained mashing of both buttons and
the switch, with no hang and no panic. That is good evidence but not proof:
the interleaving cannot be forced, and the same build was not run with `spin`
for comparison.

**Hardware futures as effects.** Both of the spike's first firmwares await
their timers as effects. The counter's flash and the HTTP firmware's stream
back-off each ask the shell for a custom `Delay` operation. That operation was
a stand-in: the proposed shape is `crux_time`'s `NotifyAfter` (design
section 8), which a shell resolves in the same way. In the counter, the
shell's main loop resolves it.
To check that this works when the timer is awaited away from the main loop,
the HTTP firmware was changed to resolve `Delay` from a separate embassy
task. The app was not changed. In the firmware:

- The `Core` is shared, as a `&'static`, by the main loop and the delay task.
- The main loop passes each `Delay` request to the delay task through a
  queue.
- The delay task awaits an embassy `Timer` for the earliest request and
  calls `Core::resolve`, which resolves it, runs the core forward and returns
  the follow-up effects (here, the request that reopens the stream). It
  passes them back to the main loop through a second queue.
- The main loop waits on the BLE link, the buttons, the switch and that
  queue, and handles those effects like any others.

Both queues are a `VecDeque` behind embassy's critical-section mutex, with
an embassy `Signal`. They are unbounded, so neither task can block waiting
for the other.

This needs no new Crux API, no root waker and no `generic-queue-N`, because
the delay task polls the `Timer` with its own embassy waker. A handler that
polls the hardware future with a different waker, as a combinator such as
`FuturesUnordered` does, is rejected by embassy's default timer queue, just
as `CommandWaker` is. `embassy_futures::select` passes the task's waker
through.

On the board, against the live server, the GET, the stream and six button
POSTs worked as before. With the network down, the server closed the
stream. The board reopened it by itself three times, each closed at once,
and the fourth held and delivered updates again. Both tasks run
cooperatively on one executor, so this run says nothing about lock
contention. It was short, manual testing.

Release text grew by 1,504 bytes, and data by 64 bytes for the two queues.
Most of the text is the delay task's own inlined copy of `Core::resolve`.
For an app with one hardware effect, like the first counter, the main loop
alone is simpler and smaller. A task per peripheral pays off when the main loop
already has a lot to wait on, as the HTTP firmware's does with the BLE link,
or when several peripherals each hold pending requests.

**One core for every shell.** The counter firmware has since been rebuilt as a
shell over the counter example's own `shared` crate, built without `std`, with
the layout in design section 9. Its LED flash is now an embassy `Timer` in the
shell, so the core has no `Delay` effect. The core's count is unbounded, so
the shell draws it as an odometer, each lap of ten pixels in a new colour.
On the board, the buttons, the brightness switch and the LED flash all work:

| | text | data | bss |
|---|---|---|---|
| Own app with a `Delay` effect | 20,728 B | 24 B | 34,036 B |
| Shell over the counter's `shared`, count clamped to ten | 19,688 B | 24 B | 34,036 B |
| The same, drawn as an odometer | 19,824 B | 24 B | 34,036 B |

On a 64-bit host, peak heap for a burst of twenty presses fell from 58,480
bytes to 3,264, because each press now ends with its render and no `Delay`
command is left in flight. These are not comparable with the 32-bit figures
above. The counter's `shared` crate is also checked for
`thumbv7em-none-eabihf` alongside its usual checks, and the spike's examples
CI job installs that target.

The `counter_http` firmware has since been rebuilt the same way, over that
example's `shared` with `crux_time` (design section 9):

| | text | data | bss |
|---|---|---|---|
| Own app copy, `Delay` task | 328,040 B | 4,256 B | 80,392 B |
| Shell over `counter-http/shared`, `crux_time` | 337,096 B | 4,264 B | 80,400 B |

Most of the 9,056 bytes is likely `chrono`: the shared view formats the
update time into `text`, which this shell never reads. `crux_time` itself is
0.4 KiB. Heap, probed on a 32-bit host, is within 2% of the copy's (peak
46,135 B against 45,435 B for twenty presses in flight), except that after a
disconnect 3,612 B stays live against 2,424 B, because the core now holds its
back-off timer.

On 2026-10-09 it was flashed and run with the gateway. The count, pending
state on presses, link-down and reconnect, and stream recovery with Wi-Fi off
(four retries, then updates again without a Bluetooth reconnect) all worked.

## Open questions

1. **The lock.** The spike has now tried the recommended split:
   `critical-section` for the internal locks and a try-lock that panics on
   contention for `Core`'s own. It worked on hardware and cost nothing
   measurable (see [Evidence](#evidence)). What is left to decide:
   - Is "one execution context owns the `Core`" the right rule on multi-core
     chips? An app that wants to call `view()` on one core while the other
     is in `process_event` would need a blocking lock. Across cores that is
     correct, but a blocking lock cannot tell the other core from an
     interrupt on its own core, so it brings back the interrupt deadlock.
   - Is the `critical-section` feature, which something in every `no_std`
     build must enable, acceptable? The alternative is an unconditional
     dependency.
   - Is `try-lock` acceptable as a dependency, or should `crux_core` allow
     the one small `unsafe` block it replaces?
2. **The channel on multi-threaded paths.** Replacing crossbeam in std builds
   too gives one implementation and full test coverage, but it needs a
   performance check on the middleware paths and a review of its disconnect
   semantics by someone who knows the executor well.
3. **The bridge without `std`.** Should it stay std-only, or move to a format
   such as bincode 2 or postcard that works without `std`? That decides
   whether a non-Rust `no_std` shell, such as C firmware over FFI, is ever
   possible. A new format would need a byte-for-byte check against the
   facet_generate runtimes.
4. **Gating `#[effect(facet_typegen)]`.** Silently omit the FFI items without
   `bridge`, or fail with a clear error? The example layout in design
   section 9 relies on the omission (see section 6).
5. **Middleware on microcontrollers.** Port it with the stricter guard, or
   gate it on `std`?
6. **Heap behaviour.** Allocation per command, and capacity retained in slabs
   and queues, should be documented. Is a long-running fragmentation test on
   a real embedded allocator needed before calling this supported?
7. **CI targets.** Is `thumbv7em-none-eabihf` alone enough, or should a
   RISC-V target such as `riscv32imac-unknown-none-elf` be built too?
8. **Capability crates.** Is a `no_std` subset with twin types, as in
   `crux_http`, acceptable, or should each crate make its public types its
   own so that one API serves both modes? For `crux_http` that is a breaking
   change and would need its own RFC; it would also become unnecessary if
   `http` gains a `no_std` mode upstream. Does `crux_kv` need a subset at
   all?
9. **Futures that are not effects.** A `Command` task can await any future,
   but when a hardware future, such as an embassy timer or a GPIO edge,
   wakes it, nothing tells the shell to call `Core` again. Exposing a root
   waker on `Core` would fix that, but it is not proposed here.

   The spike shows it is not needed if hardware futures are effects. The
   firmware awaits the hardware future for the request, wherever it likes,
   and resolves it with `Core::resolve`. That call already runs the core
   forward and returns the follow-up effects, so no new Crux API is needed
   (see [Evidence](#evidence)). Two rules apply:
   - The code that awaits the hardware future must poll it with its own
     task's waker. Then embassy's default timer queue accepts it.
   - `Core::resolve` must be called from the context that owns the `Core`,
     for example a task on the same embassy executor as the shell's main
     loop, never from an interrupt handler, where it could hit the
     try-lock's contention panic.

   For time, the effect is `crux_time`'s `NotifyAfter`, so a core needs no
   operation of its own (design section 8). Should "hardware futures are
   effects" be the documented rule? The alternative is to support a
   `Command` awaiting a hardware future directly, which would need some way
   for the shell to hear about the wake.

## Next steps

If this direction is agreed, the work splits into small pull requests, each
useful on its own:

1. Fix the `crux_core::macros` gate.
2. Move workspace dependencies to `default-features = false`, with members
   opting in, and exclude `examples` from the root workspace.
3. Replace `crossbeam-channel` and `futures::channel::mpsc` with the internal
   channel, in std builds too, with a performance comparison.
4. Add the `sync` shim: closure-based `critical-section` locks for the
   internals, try-locks for `Core`, and the `critical-section` feature.
5. Add the `bridge` feature and the macro gate.
6. Add the `no_std` attribute, the prelude and path changes, the middleware
   changes, and a CI job that builds `crux_core` for `thumbv7em-none-eabihf`.
7. Split the counter example's FFI into its own crate and gate its `shared`
   on `std`, with CI checking `shared` for `thumbv7em-none-eabihf`. Add the
   CPB shell at `examples/counter/cpb`, and the experimental Embedded page in
   Part III of the book (both done in the spike).
8. Add the `no_std` subsets of `crux_http` and `crux_time` (both done in the
   spike), and fix `facet-generate-attrs`' `facet` dependency.
9. Do the same for `counter_http`: restart-with-backoff and an error state in
   its core, its firmware at `examples/counter-http/cpb`, and the Bluetooth
   protocol and gateway in `examples_support/` (all done in the spike).
10. Later: targets without compare-and-swap once facet supports them.
