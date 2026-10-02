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
