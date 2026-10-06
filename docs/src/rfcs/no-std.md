# RFC: `no_std` + `alloc` support

```admonish
This RFC is **proposed**. It contains no implementation code. The evidence in
it comes from a throwaway spike, which is not part of this pull request, that
built `crux_core` for a Cortex-M4 and linked a small Crux app into firmware for
the Adafruit Circuit Playground Bluefruit. That firmware has since been flashed
to the board and works as intended, first with a `spin` lock and then with the
`critical-section` design this RFC recommends. Heap use was measured on a host,
not on the device.
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
with two buttons, a slide switch, a red LED and ten NeoPixels. A counter app
on it uses a render effect, a custom async `Delay` operation resolved by the
shell, and a `Command::new` task that awaits it and sends a follow-up event.
That is a realistic slice of Crux, and the spike shows it fits comfortably in
the chip's 1 MB of flash and 256 KB of RAM.

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
- The capability crates. `crux_http`, `crux_kv` and `crux_time` are not
  touched by this RFC.
- Running without a heap.

## Design

Each subsection below is one blocker, with the fix the spike used and what it
costs.

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
`std` without the bridge.

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
user and leaves the snapshots readable. This RFC leans towards the hard error;
open question 4 asks for a decision.

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
| `#[effect(facet_typegen)]` | FFI enum and `EffectFFI` impl | omitted in the spike, an error proposed |

`serde` is still compiled into firmware, because `crux_core` derives
`Serialize` and `Deserialize` on `RenderOperation` and similar types
unconditionally. It costs about 270 bytes in the spike's firmware, and could
become optional along with the bridge later.

## Drawbacks

**Workspace dependencies have to change.** `facet.workspace = true` and
`serde.workspace = true` cannot be used by a `no_std` `crux_core`, because the
workspace entries enable the crates' default features and a member cannot turn
them off. The spike spelled out versions in `crux_core`. The proper fix is
`default-features = false` in `[workspace.dependencies]`, with every member
that needs `std` opting back in, which touches every crate in the repository.

**A feature that every `no_std` crate has to name.** Cargo cannot express
"this dependency only when a feature is off", which is why the spike's first
lock, `spin`, was unconditional and sat in every app's lockfile. The
`critical-section` feature avoids that, and a plain std app's lockfile no
longer lists any lock crate. It has its own costs, which the spike found:

- Every `no_std` crate that depends on `crux_core` must enable the feature,
  including library crates that never link a binary, or that crate fails the
  `compile_error!` when built on its own. A capability crate such as
  `crux_http` would need a feature that forwards to it.
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
`thumbv7em-none-eabihf`, then linked a counter app into `embassy-nrf` firmware
for the Circuit Playground Bluefruit. It was built on a branch close to master;
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

**On hardware.** The firmware was flashed through the board's UF2 bootloader
and runs as intended. Button and switch interrupts reach the `embassy`
executor, and each press updates the model and redraws the NeoPixels. The
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

The second image is a larger spike firmware that runs the `counter_http`
example's app and reaches the network through a Web Bluetooth page. Data and
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
  propose exposing a root waker, because making the timer an effect is
  enough; see below and open question 9.
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

**Hardware futures as effects.** Both firmwares await their timers as
effects. The counter's flash and the HTTP firmware's stream back-off each ask
the shell for a `Delay`. In the counter, the shell's main loop resolves it.
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
For an app with one hardware effect, like the counter, the main loop alone
is simpler and smaller. A task per peripheral pays off when the main loop
already has a lot to wait on, as the HTTP firmware's does with the BLE link,
or when several peripherals each hold pending requests.

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
   - Is the `critical-section` feature, which every `no_std` crate in the
     graph must enable, acceptable? The alternative is an unconditional
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
   `bridge`, or fail with a clear error?
5. **Middleware on microcontrollers.** Port it with the stricter guard, or
   gate it on `std`?
6. **Heap behaviour.** Allocation per command, and capacity retained in slabs
   and queues, should be documented. Is a long-running fragmentation test on
   a real embedded allocator needed before calling this supported?
7. **CI targets.** Is `thumbv7em-none-eabihf` alone enough, or should a
   RISC-V target such as `riscv32imac-unknown-none-elf` be built too?
8. **Capability crates.** `crux_time` has no `Instant` or `SystemTime`
   without `std`. Which of `crux_time`, `crux_kv` and `crux_http` should
   follow, and how?
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

   Should "hardware futures are effects" be the documented rule? The
   alternative is to support a `Command` awaiting a hardware future
   directly, which would need some way for the shell to hear about the
   wake.

## Next steps

If this direction is agreed, the work splits into small pull requests, each
useful on its own:

1. Fix the `crux_core::macros` gate.
2. Move workspace dependencies to `default-features = false`, with members
   opting in.
3. Replace `crossbeam-channel` and `futures::channel::mpsc` with the internal
   channel, in std builds too, with a performance comparison.
4. Add the `sync` shim: closure-based `critical-section` locks for the
   internals, try-locks for `Core`, and the `critical-section` feature.
5. Add the `bridge` feature and the macro gate.
6. Add the `no_std` attribute, the prelude and path changes, the middleware
   changes, a small `no_std` example, and a CI job that builds it for
   `thumbv7em-none-eabihf`.
7. Later: the capability crates, and targets without compare-and-swap once
   facet supports them.
