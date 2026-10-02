# RFC: `no_std` + `alloc` support

```admonish
This RFC is **proposed**. It contains no implementation code. The evidence in
it comes from a throwaway spike, which is not part of this pull request, that
built `crux_core` for a Cortex-M4 and linked a small Crux app into firmware for
the Adafruit Circuit Playground Bluefruit. The firmware has not yet been run on
hardware, so everything below about runtime behaviour on the device is still
pending.
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

The spike added a crate-private `sync` module with `Mutex` and `RwLock` types.
Under `std` they are thin wrappers over `std::sync` whose `lock()`, `read()`
and `write()` return the guard and panic on poison, as every call site already
did. Without `std` they are re-exports of `spin::Mutex` and `spin::RwLock`,
which have the same method names and no poisoning. Call sites lose their
`.expect`, and `crux_core` keeps `unsafe_code = "forbid"`.

That is the shape the spike measured, and it compiles and links. The open
question is which lock to use without `std`. There are two candidates.

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
it**, for three reasons:

1. it is safe when wakers fire from interrupt handlers, which the executor's
   design makes likely as soon as a task awaits a hardware future;
2. it is what the embedded ecosystem, embassy included, already expects, so a
   firmware app already has an implementation linked;
3. it works on chips without compare-and-swap, where `spin` does not compile,
   and `portable-atomic`, which those chips would need, wants a
   `critical-section` implementation too.

It has costs, and they shape the shim:

- The shim becomes closure-based (`with_lock(|value| ...)`) in both builds,
  which touches every call site, rather than returning a guard.
- `RwLock` collapses into an exclusive lock. With one core and one executor
  there is no reader concurrency to lose.
- A critical section masks interrupts for as long as it lasts. That is fine
  for the channel and the registries, which hold their locks for a few
  instructions, but not for `Core`'s own two locks: `process_event` holds the
  model lock across `App::update`, and `process` holds the root command lock
  across the whole loop of updates and task polls. Wrapping those in a
  critical section would mask interrupts for the entire update.

So the proposed split is `critical-section` for the short-held internal locks,
and, for `Core`'s model and root command, a non-blocking lock that panics on
contention without `std`. That turns "a `Core` is used from one execution
context at a time" into a checked rule, which a firmware shell like the spike's
satisfies naturally. The final choice is kept as open question 1.

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
| Locks | `std::sync`, panic on poison | see design section 3, no poisoning |
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

**The lock crate is compiled in std builds too.** Cargo cannot express "this
dependency only when a feature is off", so the spike's `spin` is
unconditional and appears in every app's lockfile without being used. With
`critical-section` this can be avoided: a `critical-section` feature that a
`no_std` app turns on, and a `compile_error!` when neither it nor `std` is
enabled. That is one more feature for firmware authors to know about, but it
keeps std builds exactly as they are.

**A second build mode to keep green.** Without a CI job that builds
`crux_core` and a small example for `thumbv7em-none-eabihf` on every pull
request, the `no_std` path will break quietly within weeks.

**Macro snapshots get harder to read**, if the bridge gate wraps the
generated items in a `macro_rules!` invocation (design section 6).

**The internal channel is new code on a hot path**, replacing a well-tested
crate, with its performance under contention not yet measured.

## Migration

Apps on the default features notice nothing, apart from one more small
dependency in their lockfile if the lock crate stays unconditional.

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

**On hardware.** Nothing has been flashed yet. NeoPixel timing, interrupt
forwarding through the SoftDevice and button polarity are unverified.

## Open questions

1. **The lock.** `critical-section`, as recommended in design section 3, or
   `spin` with a documented rule that no Crux code runs in an interrupt
   handler? And for `Core`'s own locks, is a lock that panics on contention
   the right tool?
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
9. **Runtime verification.** The firmware still has to be run on the board,
   and the results folded into this RFC.

## Next steps

If this direction is agreed, the work splits into small pull requests, each
useful on its own:

1. Fix the `crux_core::macros` gate.
2. Move workspace dependencies to `default-features = false`, with members
   opting in.
3. Replace `crossbeam-channel` and `futures::channel::mpsc` with the internal
   channel, in std builds too, with a performance comparison.
4. Add the `sync` shim with the chosen lock.
5. Add the `bridge` feature and the macro gate.
6. Add the `no_std` attribute, the prelude and path changes, the middleware
   changes, a small `no_std` example, and a CI job that builds it for
   `thumbv7em-none-eabihf`.
7. Later: the capability crates, and targets without compare-and-swap once
   facet supports them.
