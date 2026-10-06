# Spike: Crux `no_std` + `alloc` on the Circuit Playground Bluefruit

Throwaway spike. Evidence for an RFC, not mergeable code.

## 1. Initial error inventory (before any fixes)

Change: `std = []` feature added (in `default`), `#![cfg_attr(not(feature = "std"), no_std)]`
and `extern crate alloc;` in `crux_core/src/lib.rs`. Nothing else.

### Host: `cargo check -p crux_core --no-default-features`

142 errors, all in name resolution (rustc stops before type checking, so this is
a floor, not the full list):

| Count | Error | Cause |
|---|---|---|
| 73 | `cannot find type Vec` | std prelude gone |
| 22 | `cannot find type Box` | std prelude gone |
| 42 | `cannot find module or crate std` | `use std::...` paths (36 `use` lines + 6 inline `std::mem::replace`, `std::fmt`, `std::any::type_name`, `std::error::Error`) |
| 3 | `cannot find macro vec` | `bridge/mod.rs`, `middleware/bridge.rs` |
| 2 | `cannot find macro eprintln` | `middleware/effect_handling.rs:305,333` |

Note that on the host the *dependencies* (crossbeam-channel, bincode, serde_json,
futures with default features) still build, because the host has `std`. Their
`no_std` problems are invisible here.

### Target: `cargo build -p crux_core --no-default-features --target thumbv7em-none-eabihf`

Fails before reaching crux_core. `futures-core`, `futures-sink`, `futures-task`,
`futures-io` and `memchr` all fail with `E0463 can't find crate for std`: crux_core
depends on `futures = "0.3"` with default features (std). So the honest thumb
baseline needs dependency features fixed first (section 2).

### Target baseline after fixing dependency features

With `futures`/`serde`/`facet`/`slab`/`thiserror` at `default-features = false`
(plus `alloc`), the next blocker is `crossbeam-utils` (`E0463 can't find crate
for std`): crossbeam-channel has no `no_std` mode, confirmed. With crossbeam,
bincode and serde_json made optional (std / bridge only), crux_core itself is
reached and fails with 162 errors:

| Count | Error |
|---|---|
| 80 + 12 + 3 | `Vec` / `Box` not in scope (prelude) |
| 42 | `use std::...` |
| 3 + 2 | `vec!`, `eprintln!` |
| 10 | `crossbeam_channel` unresolved (command/mod.rs, executor.rs, context.rs, stream.rs) |
| 1 | `futures::channel::mpsc` unresolved: confirmed std-only in futures-channel 0.3.33 |
| 6 | `bincode` unresolved (bridge) |
| 3 | `serde_json` unresolved (bridge) |

So the survey was right about every hard blocker; the compiler found none it missed
(at the name-resolution stage; later stages are covered below).

## 2. Result in one paragraph

`crux_core` (with `crux_macros`) builds for `thumbv7em-none-eabihf` with
`--no-default-features`, and a real Crux app (button events, a render effect,
a custom async `Delay` request resolved by the shell, a `Command::new` task
that awaits it and sends a follow-up event) links into embassy-nrf firmware:
**21.7 KB flash, 24 B .data, 34 KB .bss (32 KB of it the heap)**, of which
crux_core is about 5 KB. With default features, `cargo test --workspace` passes
(483 tests) after updating the `#[effect]` macro snapshots, and
`examples/counter` still builds and passes its tests. `thumbv6m-none-eabi` does
not build (CAS atomics; section 9). The firmware runs correctly on a real
Circuit Playground Bluefruit (flashed by UF2, section 11).

## 3. Changes to crux_core / crux_macros, by blocker

All edits are marked `spike(no_std)` where they are not self-explanatory.

### 3.1 std prelude and `std::` paths (142 errors): clean

- `crate::prelude` in `lib.rs` re-exports `Box`, `Vec`, `String`, `ToString`,
  `ToOwned` from `alloc`; `#[allow(unused_imports)] use crate::prelude::*;` is
  added to the 16 modules that needed it.
- `std::{fmt,mem,any,future,pin,task,ops,marker,sync::atomic}` became `core::`;
  `std::sync::{Arc,Weak}`, `std::collections::VecDeque` and `std::task::Wake`
  became `alloc::`. Done by script, excluding comments, test modules and the
  std-only modules (`testing.rs`, `bindgen.rs`, `type_generation/`).
- `bridge/` was rewritten to `core::` up to the first `#[cfg(test)]` only, and
  still uses `std::collections::HashMap`; it is std-only (3.4) so that is moot.
- Proper version: the same, plus a CI job so it does not regress. A clippy lint
  (`std_instead_of_core`, `std_instead_of_alloc`, `alloc_instead_of_core`) would
  keep it clean mechanically.

### 3.2 crossbeam-channel and `futures::channel::mpsc`: clean-ish, needs review

- Both replaced by one internal unbounded MPSC channel, `crux_core::sync::channel`
  (`Arc<Inner>` holding `Mutex<VecDeque<T>>`, a sender count, a
  receiver-alive flag and an `AtomicWaker`; ~150 lines). Used in **both** std and
  no_std builds, so the existing test suite exercises it, and
  `crossbeam-channel` is no longer a dependency of crux_core (kept as a
  dev-dependency because some tests use it directly).
- The disconnect semantics the executor relies on are kept:
  `send` fails once the receiver is dropped (`JoinHandle::poll` reads that as
  "task gone"); dropping the receiver drops everything still queued (wakers and
  resolve callbacks are released, which feeds the `Arc::strong_count(&waker) < 2`
  cancellation heuristic in `run_task`); dropping the last sender wakes the
  async receiver, which then yields `None` without re-registering.
- Hack-level details: `SendError` has a hand-written `Debug` (so `.expect()`
  works without `T: Debug`, as crossbeam's did). `ShellStream`'s public variants
  now carry `crux_core::sync::channel::Receiver` instead of
  `futures::channel::mpsc::UnboundedReceiver`, so `sync` is `#[doc(hidden)] pub`.
  That is a (tiny) public API change; a proper version would make `ShellStream`
  opaque.
- Risks: performance under contention was not measured (one lock per send/recv
  versus crossbeam's lock-free list); the old `// TODO: consider switching to
  flume` is now moot. The lock is held only around `VecDeque` operations, and
  destructors of drained items run outside it.

### 3.3 `std::sync::{Mutex, RwLock}`: clean

- `crux_core::sync::{Mutex, RwLock}`: under `std`, thin wrappers over
  `std::sync` whose `lock()`/`read()`/`write()` return the guard and panic on
  poison (as every call site did, just with one message now); under no_std,
  `spin::{Mutex, RwLock}` re-exported (same method names, no poisoning).
  **Superseded**: section 12 replaces `spin` with `critical-section` for the
  internal locks and a try-lock for `Core`'s.
- The `.expect("... poisoned")` at 9 call sites in `core/mod.rs`,
  `bridge/registry.rs`, `effects/registry.rs`, `effects/routes/buffer.rs` went.
- No `unsafe`: `unsafe_code = "forbid"` still holds for crux_core.
- Proper version: decide whether a spinlock is the right no_std lock. It is
  fine while `Core` is only touched from one executor (the shell here), but a
  spinlock taken from an interrupt handler while thread mode holds it deadlocks
  on a single core. `critical-section` (`critical_section::Mutex<RefCell<T>>`)
  is the embedded-standard answer, but its API is `with(|cs| ...)` closures, not
  guards, so the shim would need a different shape.

### 3.4 bincode 1.3 / serde_json / `HashMap`: gated (clean as a first step)

- New `bridge` feature (`["std", "dep:bincode", "dep:serde_json"]`), in
  `default`. Gated on it: `mod bridge`, the `EffectFFI` trait and re-export,
  `middleware::Bridge` + `Layer::bridge` + the format re-exports, and
  `effects::routes::Serialized` (it is built on `bridge::ResolveRegistry`).
- Proper version: either keep the bridge std-only (an embedded shell calls
  `Core` directly, as here), or move to bincode 2 / postcard with a
  byte-for-byte check against the facet_generate runtimes.

### 3.5 Reentrancy guard and `eprintln!` in `middleware/effect_handling.rs`: hack

- The `ThreadId` half of the guard is `#[cfg(feature = "std")]`. Without std the
  guard is the `AtomicBool` alone, so **any** `resolve()` while
  `try_process_effect` is on the stack panics, including one from another core
  or an ISR. Stricter than std, which only panics for the same thread.
- `eprintln!` is behind `crate::__crux_log_error!`, which prints under std and
  drops the message otherwise. Proper version: use `log` (already optional).

### 3.6 Macro output: one clean fix (see section 5)

- `crux_core::__crux_core_bridge_items!` (modelled on the existing
  `__crux_core_testing_items!`) expands to its input only when crux_core has
  `bridge`. `#[effect]` now wraps the generated `<Effect>Ffi` enum and the
  `EffectFFI` impl in it.
- Snapshot tests in `crux_macros/src/effect/tests.rs` were re-accepted
  (`cargo insta test -p crux_macros --accept`), and
  `the_largest_effect_enum_the_id_can_describe` now compares with whitespace
  stripped, because prettyplease prints the tokens inside a macro invocation
  verbatim. That is the real cost of this approach: the generated FFI code in
  the snapshots is no longer pretty-printed.

### 3.7 Pre-existing bug found on the way: clean fix

- `lib.rs` had `#[cfg(feature = "default")] pub use crux_macros as macros;`,
  so `crux_core::macros` disappeared with `default-features = false` even when
  `crux_macros` was enabled. Now `#[cfg(feature = "crux_macros")]`. This
  affects std users who turn default features off today, not just no_std.

## 4. Feature layout and std / no_std differences

```toml
[features]
default = ["std", "bridge", "crux_macros"]
std = ["facet/std", "futures/std", "serde/std", "slab/std", "thiserror/std"]
bridge = ["std", "dep:bincode", "dep:serde_json"]
testing = ["std", "dep:anyhow"]
facet_typegen = ["bridge", "crux_macros/facet_typegen", ...]  # needs serde_json and bridge::Request
uniffi_compat_bindgen = ["std", "dep:anyhow", ...]
```

All of `std`, `std,crux_macros`, `crux_macros`, `testing`,
`uniffi_compat_bindgen`, `facet_typegen` and the empty set `cargo check` on the
host. `facet_typegen` first failed with only `std` (it uses `serde_json`, and
the `#[effect(facet_typegen)]` `Export` impl names `bridge::Request<EffectFfi>`),
so it implies `bridge`. Note `bridge` is a separate default feature rather
than implied by `std`, so `std` without the bridge is expressible.

Behaviour differences without `std`:

| | std | no_std |
|---|---|---|
| Internal locks (channel, registries) | `std::sync`, panic on poison | `critical-section`, masks interrupts briefly (section 12) |
| `Core`'s model and root command | `std::sync`, blocks | try-lock, panics on contention (section 12) |
| Middleware reentrancy guard | panics on same-thread resolve during `try_process_effect` | panics on any resolve during it |
| Middleware diagnostics | `eprintln!` | silent |
| `bridge`, `EffectFFI`, `Serialized` route, `middleware::Bridge` | yes | absent |
| `testing`, `facet_typegen`, `uniffi_compat_bindgen` | available | require std |
| `#[effect(facet_typegen)]` | FFI enum + `EffectFFI` | both silently omitted |

## 5. Dependency changes

- **Removed** from crux_core: `crossbeam-channel` (now a dev-dependency only).
- **Added**, then replaced (section 12): `spin 0.9` (`mutex`, `spin_mutex`, `rwlock`, no default features).
  It is unconditional, so std builds compile it without using it (one extra
  small crate in every app's lockfile; `examples/counter/Cargo.lock` changes
  accordingly). Cargo has no "only when a feature is off" dependency; a proper
  version would add a `spin` feature, or accept it.
- **Made optional**: `anyhow` (testing / bindgen), `bincode`, `serde_json`
  (bridge).
- **default-features = false**: `futures` (+ `alloc`), `serde` (+ `derive`,
  `alloc`), `facet` (+ `alloc`), `slab`, `thiserror`; their std features come
  back through crux_core's `std`.
- **Surprise: workspace dependencies.** `facet.workspace = true` and
  `serde.workspace = true` cannot be used, because the workspace entries enable
  default (std) features and a member cannot turn them off. crux_core now
  spells out versions. A proper version sets `default-features = false` in
  `[workspace.dependencies]` and has every member opt in.
- `futures::channel::mpsc` really is std-only (futures-channel 0.3.33,
  `#[cfg(feature = "std")] pub mod mpsc`). `BoxFuture`, `AtomicWaker`,
  `FuturesUnordered`, `Stream`/`Sink`, `StreamExt` all work with `alloc`.
- facet 0.46.5 with `default-features = false, features = ["alloc"]` was
  unproblematic on thumbv7em, in crux_core (`RenderOperation`, `ResolveError`
  derive it) and in the user crate (section 6). On thumbv6m it fails (section 9).
- serde is still compiled into the firmware (~270 B) because crux_core derives
  `Serialize`/`Deserialize` on `RenderOperation` and friends unconditionally.
  Could become optional with the bridge.

## 6. Macro output findings

Tested against the real firmware crate (`#![no_std]`, edition 2024):

- `#[derive(Operation)]` / `#[operation(request, output = ())]`: no changes
  needed. Output uses `::core::option::Option` and `::crux_core::...` paths.
- Plain `#[effect]`: **worked unchanged** even before the macro edit. It emits
  `From`/`TryFrom` impls, `is_*`/`into_*` helpers using bare `Option`, `Some`,
  `None`, `Result`, `Ok`, `Err` (all in the core prelude), and the testing
  items behind `__crux_core_testing_items!`. No `Vec`/`Box`/`String`/`format!`.
  (Minor: it uses `crux_core::Effect` without a leading `::`; and bare
  `TryFrom` would break a 2018-edition crate, which is not a real concern.)
- `#[effect(facet_typegen)]` with the **original** macro, in the no_std crate:
  5 errors: `cannot find trait EffectFFI in crate crux_core`, `could not find
  bridge in crux_core`, `could not find serde in the list of imported crates`,
  `cannot find attribute serde`, and a knock-on `no variant named Ffi`. With
  the `__crux_core_bridge_items!` gate it compiles, and the FFI enum is simply
  not generated (plus a harmless `unexpected cfg value: facet_typegen` warning
  from the `cfg_attr` in the user crate, which has no such feature).
- `#[derive(Facet)]` in the no_std user crate (on `Rgb` and `ViewModel`)
  compiles with `facet = { default-features = false, features = ["alloc"] }`:
  facet's derive output does not reference `::std`.
- The `Capability` derive was not exercised (deprecated path).

## 7. Firmware

### Crates

`embassy-nrf 0.11` (`nrf52840`, `time-driver-rtc1`, `gpiote`, `time`),
`embassy-executor 0.10` (`platform-cortex-m`, `executor-thread`),
`embassy-time 0.5.1`, `embassy-futures 0.1.2`, `cortex-m 0.7`
(`critical-section-single-core`), `cortex-m-rt 0.7.7`, `embedded-alloc 0.7`
(`LlffHeap`), `panic-halt 1.0`. No defmt (no probe assumed). The crate is a
standalone workspace with `.cargo/config.toml` (target, `-Tlink.x`),
`build.rs` (copies `memory.x`) and `memory.x`.

### Pins (verified)

From CircuitPython `ports/nordic/boards/circuitplayground_bluefruit/pins.c`,
`mpconfigboard.h` and `board.c`, and Adafruit_CircuitPlayground's pin modes:
button A P1.02, button B P1.15 (both active high, `INPUT_PULLDOWN`), slide
switch P1.06 (`INPUT_PULLUP`), red LED D13 = P1.14 (active high), 10 NeoPixels
on P0.13. **Also P0.06 is a power switch: it must be driven low, or the
NeoPixels (and sensors) have no power**; CircuitPython's `board_init` does this.
The firmware drives it low.

### What the app does

- `Event::{ButtonA, ButtonB, Switch(bool), FlashDone(u32)}`;
  `Effect::{Render(RenderOperation), Delay(Delay)}` via `#[effect]`;
  `Delay { millis }` via `#[derive(Operation)]`, a `request` with output `()`.
- A press changes the count (clamped to ±10), lights `|count|` NeoPixels
  (green for positive, red for negative; the slide switch picks bright/dim),
  and returns `render().and(Command::new(...))`. The async task
  `ctx.request_from_shell(Delay { millis: 120 }).await`, then
  `ctx.send_event(FlashDone(id))`. The red LED is on while a flash is pending.
- The shell (`main.rs`) is one embassy task: `select4` over the debounced
  level changes of the two buttons and the switch, and a `Timer::at` for the earliest
  outstanding `Delay`. It keeps `Request<Delay>` values with their due
  `Instant`, and calls `core.resolve(&mut request, ())` when due, feeding the
  returned effects back through the same handler.
- NeoPixels: WS2812 bits from `SequencePwm` (PWM0, 16 MHz, 20 ticks per bit,
  T0H 7 / T1H 13, ~50 µs reset), following embassy's
  `pwm_sequence_ws2812b` example, with HFCLK from the external crystal for
  timing. The buffer is 241 `u16` in RAM (EasyDMA requirement). After starting
  the sequence the shell busy-waits ~1 ms before dropping the sequencer (drop
  stops it); crude but enough. **Verified on hardware (2026-10-04)**: GRB
  colour order and the 3.3 V data level work, green and red show correctly at
  both brightnesses. The red LED path is the same `render()` call.
- Debounce: each input reports only settled level changes (30 ms after an
  edge, re-checked), on press and release. The first version slept 40 ms
  after a press only, and on hardware release bounce counted as extra presses.
  The fix is verified on hardware: taps, long holds and slow releases each
  count exactly once.

### Memory layout and its uncertainty

`memory.x` mirrors Adafruit's own Arduino linker script for this chip,
`cores/nRF5/linker/nrf52840_s140_v6.ld` (fetched, not remembered):
`FLASH ORIGIN = 0x26000, LENGTH = 0xED000 - 0x26000`,
`RAM ORIGIN = 0x20006000, LENGTH = 0x20040000 - 0x20006000`.

- The MBR forwards reset and interrupts to the SoftDevice, which (never
  enabled here) forwards them to the application at the address where *it*
  ends. So the flash origin is dictated by the SoftDevice version, not chosen:
  0x26000 for S140 6.1.1 (what the CPB ships with, and what Adafruit's current
  Arduino core still links for), 0x27000 for S140 7.x. **Confirmed: S140 6.1.1**
  (test board: UF2 bootloader 0.9.0, Board-ID nRF52840-CircuitPlayground-revD,
  dated May 9 2024).
  `INFO_UF2.TXT` on the CPLAYBTBOOT drive names the SoftDevice. If it says 7.x,
  change both `memory.x` and the `-b` address below to 0x27000.
- 0xED000..0xF4000 is left for the bootloader's user-data area, 0xF4000 up is
  the bootloader.
- RAM below 0x20006000 is left for the SoftDevice even though it is never
  enabled (strictly only the MBR's first 8 bytes are needed then). That costs
  24 KB of 256 KB, which does not matter here.
- The vector table is at 0x26000; the initial SP is 0x20040000 and the reset
  vector 0x26101 (checked in the binary).

### Size

`cargo size --release`: text 21,652 B, data 24 B, bss 34,012 B (32 KB heap,
the embassy task arena, the PWM buffer). The flashed image is 21.2 KB (UF2
42.5 KB, 512-byte blocks carrying 256 bytes each).

`cargo bloat --release --crates` (.text 20.8 KiB): core+alloc 6.8 KiB,
**crux_core 5.0 KiB**, embassy_executor 2.8 KiB, the app and shell 1.8 KiB
(which includes monomorphised crux generics), embassy_nrf 1.4 KiB,
embedded_alloc 0.7 KiB, futures_util 0.6 KiB, futures_core 0.3 KiB, serde
0.3 KiB, slab 0.1 KiB. So Crux plus its futures/slab/serde support is roughly
6–7 KiB of flash for this app.

### Heap

`heap-probe/` runs the firmware's `app.rs` unchanged (same no_std crux_core
features) on a host with a counting global allocator, drives it the way the
shell does, and reports peak live heap. On 32-bit wasm (wasm32-wasip1 under
`node:wasi`, closest to the Cortex-M4's pointer size):

| | peak heap |
|---|---|
| one press, its flash in flight | 1,956 B |
| 5 overlapping presses | 8,644 B |
| 20 overlapping presses | 29,784 B (≈1.5 KB each) |
| idle after settling | 568 B, rising to 2,600 B after the 20-burst (slab / queue capacity retained, stable across repeats, not a leak) |

On 64-bit host it is about double. Allocator overhead is not counted. A
human with a 40 ms debounce and a 120 ms flash rarely has more than 3–4
presses in flight, so 8 KB would probably do; the firmware uses 32 KB for
margin. Each `Command` costs several small allocations (four channels, an
`Arc` per task flag, a boxed future), so heap is proportional to concurrent
commands, which is worth stating in an RFC.

## 8. Tooling installed

`rustup target add thumbv7em-none-eabihf thumbv6m-none-eabi`,
`rustup component add llvm-tools`, `cargo install cargo-binutils cargo-bloat`.
`uf2conv.py` and `uf2families.json` downloaded from github.com/microsoft/uf2
(`utils/`) into a scratch directory, not committed.

## 9. thumbv6m-none-eabi (Cortex-M0, no CAS)

`cargo build -p crux_core --no-default-features --features crux_macros --target thumbv6m-none-eabi`
fails in dependencies before crux_core:

- `spin 0.9.9`: 13 errors, `compare_exchange`, `compare_exchange_weak`,
  `fetch_add`, `fetch_sub`, `fetch_and`, `fetch_or` not found on
  `AtomicUsize`/`AtomicBool` (mutex/spin.rs, rwlock.rs).
- With spin's `portable_atomic` feature turned on (measurement only, reverted)
  the errors become "requires atomic CAS but not available on this target by
  default" (portable-atomic needs `critical-section` or
  `--cfg portable_atomic_unsafe_assume_single_core`), **and** `facet-core
  0.46.5` fails: `unresolved import alloc::sync` in `impls/alloc/arc.rs`.
  facet-core uses `alloc::sync::Arc` unconditionally, so facet itself has no
  no-CAS story today.
- Not reached but certain from source: `alloc::sync::{Arc, Weak}` does not
  exist without `target_has_atomic = "ptr"`, and crux_core uses `Arc`
  throughout (Command, executor, middleware, effects); futures-util 0.3 gates
  `AtomicWaker`, `FuturesUnordered` and more on `target_has_atomic = "ptr"`.

So no-CAS targets need `portable-atomic` + `portable-atomic-util::Arc` (whose
`Arc` cannot be used as `self: Arc<Self>` receivers for `Wake`), futures'
`portable-atomic` feature, a lock that is not `spin` on bare atomics, and a
facet change upstream. Out of scope for a first pass.

## 10. Open questions and risks for an RFC

1. **Lock choice**: `spin` (simple, guard API, deadlocks if an ISR touches
   `Core`) versus `critical-section` (embedded standard, closure API, would
   reshape the shim). Is "Core is only touched from thread mode / one
   executor" an acceptable documented constraint? *Tried in section 12:
   `critical-section` for the internal locks plus a try-lock for `Core` works,
   costs nothing measurable, and makes the one-context rule a checked one.*
2. **Replacing crossbeam everywhere**, std included: one channel
   implementation is good for test coverage, but it needs a performance check
   on the multi-threaded middleware paths, and a careful review of the
   disconnect semantics (3.2) by someone who knows the executor.
3. **Bridge on no_std**: keep std-only, or move to a no_std wire format? That
   decides whether no_std shells can ever be non-Rust (e.g. C firmware over
   FFI) without std.
4. **Macro gating via `macro_rules!` in crux_core** works but loses pretty
   snapshot output, and makes `#[effect(facet_typegen)]` silently generate
   less. Alternative: a hard error with a clear message when `facet_typegen`
   is asked for without `bridge`.
5. **Workspace dependencies** need `default-features = false` at the workspace
   level, which touches every crate in the repo.
6. **Middleware semantics** differ (stricter reentrancy guard, no
   diagnostics). Is effect middleware even wanted on MCUs, or should it be
   gated on std too?
7. **Heap behaviour**: allocation per Command and retained capacity (slab,
   queues) are fine on a 256 KB part but should be documented; a fragmentation
   test on a real allocator over a long run was not done.
8. **CI**: build crux_core and a tiny no_std example (this one) for
   thumbv7em-none-eabihf on every PR, or the no_std path will rot quickly.
9. **Capability crates**: `crux_time` (no `Instant`/`SystemTime` in core),
   `crux_kv` and `crux_http` were not touched.
10. Flashed and working on a real CPB (2026-10-04): it boots at 0x26000
    through the MBR and the disabled SoftDevice, GPIOTE interrupts reach the
    app, buttons A/B, the slide switch, the NeoPixels and D13 all behave as
    described in section 7. Not measured on hardware: heap use and timing.

## 11. Rebuild and flash

`just flash` (see README.md) does all of this: build, UF2 conversion, and the copy onto
`CPLAYBTBOOT`. The steps by hand:

```sh
cd examples/cpb-counter
cargo build --release
cargo size --release                      # optional
cargo objcopy --release -- -O binary target/cpb-counter.bin
curl -sSLO https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2conv.py
curl -sSLO https://raw.githubusercontent.com/microsoft/uf2/master/utils/uf2families.json
python3 uf2conv.py target/cpb-counter.bin -c -b 0x26000 -f 0xADA52840 -o target/cpb-counter.uf2
python3 uf2conv.py target/cpb-counter.uf2 -i   # check: family NRF52840, address 0x26000
```

(Needs `rustup target add thumbv7em-none-eabihf`, `rustup component add
llvm-tools`, `cargo install cargo-binutils`.)

Heap probe: `just heap-probe` (host). Its `.cargo/config.toml` pins
`aarch64-apple-darwin` to override the parent firmware target, and the Justfile passes the
actual host target, so it runs on any machine. Or
`cargo build --release --target wasm32-wasip1` and run the `.wasm` under any
WASI runtime for 32-bit figures.

Drag and drop:

1. Plug the CPB in over USB (a data cable).
2. Double-press the small reset button in the middle of the board. The ring
   of NeoPixels turns green and a drive called **CPLAYBTBOOT** appears.
3. Optional: open `INFO_UF2.TXT` and check the SoftDevice is S140 6.x. If it
   is 7.x, rebuild with 0x27000 (memory.x and `-b`).
4. Copy `target/cpb-counter.uf2` onto CPLAYBTBOOT. The drive ejects itself and
   the board resets into the app.
5. Button A (left) adds a pixel, button B (right) removes one (negative counts
   are red), the slide switch picks the brightness, and D13 flashes on each
   press. To go back to CircuitPython, double-press reset and copy its UF2.

## 12. Lock experiment: `critical-section` + try-lock (2026-10-06)

This tries the design recommended in the RFC (`rfc/no-std`, design section 3) in
place of `spin`.

### Shape

- **Internal locks** (the channel queue, `EffectRegistry`, `Buffer`, and the
  bridge's `ResolveRegistry`) use `crux_core::sync::Mutex`. It is closure-based
  in both builds: `with(|value| ...)`. Under `std` it wraps `std::sync::Mutex`.
  Without `std` it is `critical_section::Mutex<RefCell<T>>` inside
  `critical_section::with`. If the closure re-enters the same lock, the
  `RefCell` borrow panics rather than deadlocking.
- **`Core`'s model and root command** use `CoreRwLock` / `CoreMutex`, which
  keep the guard API, so `core/mod.rs` and its `runtime.md` anchors barely
  change. Under `std` they block, as before. Without `std` they are the
  `try-lock` crate's `TryLock`, `.expect`ed with "Core used from two execution
  contexts at once". `read()` is exclusive too.
  - Interrupts stay enabled for the whole of `App::update`.
  - The model is borrowed in place, never moved.
  - Nothing anywhere busy-waits on Crux's own locks.
  - Checking the model out (`Mutex<RefCell<Option<T>>>`: take it, run, put it
    back) was considered and rejected, because it copies the model on the
    stack on every call.
- **Feature**: `critical-section = ["dep:critical-section", "dep:try-lock"]`.
  Without `std` and without that feature, a `compile_error!` fires. The first
  error is clear, though unresolved-import errors cascade after it. `spin` is
  gone.
- **Still no `unsafe`**: `try-lock` provides the `UnsafeCell` + `AtomicBool`
  that `unsafe_code = "forbid"` stops crux_core writing itself.

### Rules this encodes

- Without `std`, a `Core` is used from one execution context: one thread,
  task or core, and never an interrupt handler. Breaking that panics, where
  `spin` would have deadlocked. It is documented on `Core` and in the
  `# Panics` sections.
- Wakers and effect resolution may run anywhere, including interrupt handlers
  and the other core of a multi-core chip. They only touch the
  `critical-section` locks.
- **Multi-core**: multi-core `critical-section` implementations (`embassy-rp`,
  `esp-hal`) take a hardware spinlock and mask interrupts on the local core.
  That is correct, but it is **one lock shared by the whole chip**. While Crux
  holds it, the other core's critical sections wait, including its allocator
  and embassy channels. So the closures must stay short. Not tested here: no
  multi-core hardware.
- With `std`, contention blocks. Host tests therefore never exercise the panic
  path; only no_std builds (the heap probe, the firmware) do.

### Work kept out of critical sections

- `EffectRegistry::resolve` takes the handle out under the lock, runs
  `handle.resolve(output)` outside it (resolving wakes tasks, which takes the
  channel lock), then reinserts it under the lock.
- `Sender::send` wakes the receiver after leaving the lock. `Receiver::drop`
  drops the drained queue outside it.
- `Buffer::drain` now `mem::take`s the vector instead of
  `drain(..).collect()`, so it doesn't allocate a new one under the lock. The
  cost is that the buffer's capacity goes with it, so the next `push` allocates.
- **Still inside**: `VecDeque::push_back` in `send` and the slab insert in
  `EffectRegistry::register` can grow, and so allocate, inside a critical
  section. `embedded-alloc` takes its own nested critical section, which is
  fine. On one core this only adds interrupt latency. On multi-core it also
  stalls the other core. Not measured.
- The bridge's `ResolveRegistry::resume` still resolves (deserialises and wakes)
  under its lock. `bridge` implies `std`, so this is always `std::sync::Mutex`
  and never an interrupt hazard.

### Measurements (same day, before and after)

| | `spin` | `critical-section` + try-lock |
|---|---|---|
| cpb-counter `.text` | 21,356 B | 20,728 B (−628 B) |
| cpb-counter-http firmware `.text` | 327,288 B | 326,432 B (−856 B) |
| cpb-counter heap probe, host, peak / idle after the 20-burst | 58,416 / 3,184 B | 58,480 / 3,248 B |
| cpb-counter-http heap probe, wasm32, peak / idle | 45,435 / 3,896 B | 45,435 / 3,896 B (identical) |

- `data` and `bss` are unchanged.
- The host probe's constant +64 B is one-time setup in the `critical-section`
  crate's `std` implementation, not Crux: on wasm the numbers are identical.
- `cargo bloat --release --crates` puts crux_core at 4.7 KiB of `.text`
  (5.0 KiB in section 7, an older build, so not a strict before/after). That
  is consistent with the saving coming from crux_core's locks, but it wasn't
  measured against the spin build on the same day. A likely reason is that on
  Cortex-M, `critical_section::with` with `critical-section-single-core` is a
  PRIMASK save, `cpsid i` and a restore, which is smaller than `spin`'s
  compare-and-swap loops.

### Friction

- **Every no_std crate that depends on crux_core needs the feature.** That
  includes library crates that never link a binary (`cpb-protocol`,
  `cpb-counter-http-app`), or they fail the `compile_error!` when built alone.
  `crux_http` without `std` now also fails on its own unless something enables
  `crux_core/critical-section`. Nothing builds it alone today, but a
  capability crate would want a forwarding feature.
- **Feature unification brings it into std builds.** The std-only gateway
  compiles `critical-section` and `try-lock` because `cpb-protocol` enables
  the feature. That is harmless: the `std` locks win, so no implementation is
  needed. But the RFC's "std builds exactly as they are" only holds where no
  no_std library is in the graph. The root workspace `Cargo.lock` also lists
  both crates, because Cargo locks optional dependencies of members.
  Re-resolving a plain std app (`examples/counter`) just removes `spin` from
  `crux_core`'s entry and adds nothing, which confirms that std apps with no
  no_std library in the graph are unaffected. The other std examples'
  lockfiles still list `spin` under crux_core until they are next resolved.
- **Host-run no_std builds need an implementation.** Both heap probes add
  `critical-section = { features = ["std"] }`. Forgetting it is a link error,
  not a silent fallback.
- The firmware already links an implementation: `cortex-m`'s
  `critical-section-single-core` in cpb-counter, and `nrf-mpsl`'s
  `critical-section-impl` in cpb-counter-http. The latter leaves MPSL's
  high-priority radio interrupts unmasked by design, so "masks interrupts"
  depends on the platform.
- **Call-site churn**: 11 call sites across `sync.rs`, `effects/registry.rs`,
  `effects/routes/buffer.rs` and `bridge/registry.rs` became closures. None
  were awkward. `Core`'s call sites are unchanged.

### Interrupt-context wakes (tried, not committed)

The committed firmware never wakes a `Command` task from an interrupt, because
its Delay is a shell request. To exercise that path, the cpb-counter flash was
changed temporarily to await `embassy_time::Timer::after_millis` inside the
`Command`. The change was flashed on 2026-10-06 and then abandoned. It needed:

- **A way for the shell to hear about the wake.** The executor only runs when
  the shell calls into `Core`, and a timer wake is not an effect. The
  `root-waker` branch's `Core::with_waker` and a public `Core::process` were
  ported for the test. The root waker raised an embassy `Signal` (critical
  section), and the main loop selected on it and called `process()`. Crux's
  public API should not have a root waker, so a shipped version needs another
  answer: for example, a shell-side async adapter, or the rule that hardware
  futures are always wrapped as effects.
- **embassy-time's `generic-queue-N`.** The default integrated timer queue
  takes the timer slot from the waker via `task_from_waker`, which panics on a
  waker the embassy executor didn't create, such as Crux's `CommandWaker`.
  The generic queue clones any waker. It cost 520 B of `.data` for 32 slots;
  when full, it wakes the earliest timer early.

The resulting wake chain runs entirely in the RTC1 interrupt handler, inside
embassy's own critical section:

1. The timer wakes the task's `CommandWaker`.
2. The waker sends on the child command's ready channel, which takes a nested
   critical section.
3. It wakes the child's `AtomicWaker`, which is the root command's
   `CommandWaker`.
4. That sends on the root's ready channel.
5. It wakes the root waker, which raises the `Signal`.

With `spin`, step 2 or 4 deadlocks if thread mode holds the same channel lock
when the RTC fires.

On hardware, every press blinked the LED for about 120 ms and back off, even
under sustained mashing of both buttons and the switch. There was no hang and
no `Core` contention panic, and the switch still worked. This is good evidence
but not proof: the exact interleaving can't be forced on this board, and the
same build was not run with `spin` for comparison.
- Hardware run (cpb-counter-http, 2026-10-06): flashed and connected through
  the Chrome gateway. The initial GET, the SSE stream (8 chunks) and four
  button POSTs (`/inc`, then `/dec` three times) all completed, and the board
  followed the shared count. There was no `Core` contention panic. This was
  short, manual testing. With the network dropped, the server closed the SSE
  stream. The board reopened it by itself four times (#7 to #10) until one
  held. A button POST then went through and the stream delivered the update.
  The plain cpb-counter firmware was not re-run on hardware.

