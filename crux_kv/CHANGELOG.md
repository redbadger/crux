# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.15.0](https://github.com/redbadger/crux/compare/crux_kv-v0.14.0...crux_kv-v0.15.0) - 2026-09-06

### 🚀 Features

- **One type per store operation, in the new `crux_kv::operation` module, and a
  `KeyValueStore` capability that sends them.** `KeyValueOperation` has one output
  type — `KeyValueResult` — for all five of its variants, so a `Get` can be answered
  with a `KeyValueResponse::ListKeys` as far as the type system and the deserializer
  are concerned, and the capability has to check:

  ```rust
  // before
  Command::request_from_shell(KeyValueOperation::Get { key })
      .map(KeyValueResult::unwrap_get) // panics if the shell answered something else
  ```

  Each operation is now its own type with exactly one output, so there is nothing to
  check:

  ```rust
  // after
  use crux_kv::{KeyValueStore, operation};

  #[effect]
  enum Effect {
      Get(operation::Get),
      Set(operation::Set),
      Render(RenderOperation),
  }

  KeyValueStore::get("key").then_send(Event::Loaded)
  ```

  `KeyValueStore` has the same five methods as `KeyValue` — `get`, `set`, `delete`,
  `exists`, `list_keys` — with the same signatures and the same `DataResult`,
  `StatusResult` and `ListResult` return types. Its bounds are per method, so an app's
  `Effect` only has to carry the operations it actually uses.

  The wire types:

  | Operation | Fields | Output | Kind |
  | --- | --- | --- | --- |
  | `Get` | `key: String` | `ValueResult` | `Request` |
  | `Set` | `key: String, value: Vec<u8>` | `ValueResult` | `Request` |
  | `Delete` | `key: String` | `ValueResult` | `Request` |
  | `Exists` | `key: String` | `BoolResult` | `Request` |
  | `ListKeys` | `prefix: String, cursor: u64` | `KeysResult` | `Request` |

  where the outputs are the usual `Ok`/`Err` pairs, since `Result` does not cross the
  FFI boundary:

  ```rust
  pub enum ValueResult { Ok(Value), Err(KeyValueError) }
  pub enum BoolResult  { Ok(bool),  Err(KeyValueError) }
  pub enum KeysResult  { Ok(Keys),  Err(KeyValueError) }

  pub struct Keys { pub keys: Vec<String>, pub next_cursor: u64 }
  ```

  Each converts to and from the `Result` alias an app sees (`ValueResult` ↔
  `DataResult`, `BoolResult` ↔ `StatusResult`, `KeysResult` ↔ `ListResult`), so a shell
  that already speaks one API can serve the other.

  The enum API is unchanged and still works — an app can use both side by side, and
  migrate one call at a time — but it is deprecated from this release; see below.

### ⚠️ Deprecated

- **The enum-shaped API is deprecated in favour of the per-operation types.** Nothing
  is removed in this release and nothing changes on the wire; each item warns, names
  its replacement, and will be removed in the next breaking release.

  | Deprecated | Since | Use instead |
  | --- | --- | --- |
  | `KeyValue` | 0.15.0 | `KeyValueStore` |
  | `KeyValueOperation` | 0.15.0 | `operation::{Get, Set, Delete, Exists, ListKeys}` |
  | `KeyValueResult` | 0.15.0 | `operation::{ValueResult, BoolResult, KeysResult}` |
  | `KeyValueResponse` | 0.15.0 | the output type of the operation you sent |

  `KeyValueError`, `Value`, `DataResult`, `StatusResult` and `ListResult` are **not**
  deprecated: both APIs share them.

  Migrating is mostly mechanical — the `KeyValueStore` methods have the same names,
  signatures and return types as `KeyValue`'s, so the work is in the app's `Effect`
  enum and in the shells that serve it. The
  [migration guide](https://redbadger.github.io/crux/guide/migrate-per-operation-types.html)
  walks through it, including the shell side.

## [0.14.0](https://github.com/redbadger/crux/compare/crux_kv-v0.13.0...crux_kv-v0.14.0) - 2026-08-06

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.20.0. No public API changes — this crate's source is
  unchanged since 0.13.0. Released in lockstep because each capability crate requires a
  specific `crux_core` minor: a capability left on `crux_core` 0.19 alongside one on 0.20
  would pull two incompatible `crux_core` versions into the same tree, with two distinct
  `Effect` and `Operation` trait sets.
- Dependency updates, and `categories` added to the crate metadata for crates.io.

## [0.13.0](https://github.com/redbadger/crux/compare/crux_kv-v0.12.0...crux_kv-v0.13.0) - 2026-05-31

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.19.0. No public API changes.
- Internal clippy nursery improvements.

## [0.12.0](https://github.com/redbadger/crux/compare/crux_kv-v0.11.0...crux_kv-v0.12.0) - 2026-05-07

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.18.0. No public API changes.

## [0.11.0](https://github.com/redbadger/crux/compare/crux_kv-v0.10.0...crux_kv-v0.11.0) - 2026-03-20

### 🚀 Features

**This is a breaking release.**

- **Command API Support**: Updated to work with the new Command API, removing dependency on the deprecated Capability trait.
- **Enhanced Testing**: Improved testing support with new command-based test helpers.

### ⚙️ Miscellaneous Tasks

- Update to `facet_generate` 0.15, `facet` 0.31, and other Rust dependencies.
- Migrate internal implementation from Capability to Command API.
- Update documentation and examples for Command-based usage.
- Update Rust dependencies.

## [0.10.0](https://github.com/redbadger/crux/compare/crux_kv-v0.9.0...crux_kv-v0.10.0) - 2025-07-31

### 🚀 Features

- *(crux_core)* Facet-based typegen frontend

### ⚙️ Miscellaneous Tasks

- *(crux)* Format fixes

## [0.9.0](https://github.com/redbadger/crux/compare/crux_kv-v0.8.1...crux_kv-v0.9.0) - 2025-05-27

This is a minor bump because of breaking changes in `crux_core`

### Other

- clippy pedantic

## [0.8.1](https://github.com/redbadger/crux/compare/crux_kv-v0.8.0...crux_kv-v0.8.1) - 2025-05-27

### Fixed

- fixes based on review feedback

### Other

- clippy pedantic

## [0.8.0](https://github.com/redbadger/crux/compare/crux_kv-v0.7.0...crux_kv-v0.8.0) - 2025-04-09

### Other

- updated the following local packages: crux_core

## [0.7.0](https://github.com/redbadger/crux/compare/crux_kv-v0.6.6...crux_kv-v0.7.0) - 2025-04-09
Replaces yanked 0.6.6 as that broke typegen on older versions of crux_core.

## [0.6.6](https://github.com/redbadger/crux/compare/crux_kv-v0.6.5...crux_kv-v0.6.6) - 2025-04-08

Updated to use version 0.13 of [`crux_core`](https://crates.io/crates/crux_core).

## [0.6.5](https://github.com/redbadger/crux/compare/crux_kv-v0.6.4...crux_kv-v0.6.5) - 2025-03-21

Patch release, no API changes.

### Other

- updated the following local packages: crux_core

## [0.6.4](https://github.com/redbadger/crux/compare/crux_kv-v0.6.3...crux_kv-v0.6.4) - 2025-03-17

This is a maintenance release (minor non-breaking changes and dependency updates).

### Other

- rust deps

## [0.6.3](https://github.com/redbadger/crux/compare/crux_kv-v0.6.2...crux_kv-v0.6.3) - 2025-02-18

### Other

- updated the following local packages: crux_core

## [0.6.2](https://github.com/redbadger/crux/compare/crux_kv-v0.6.1...crux_kv-v0.6.2) - 2025-02-03

### Other

## [0.6.1](https://github.com/redbadger/crux/compare/crux_kv-v0.6.0...crux_kv-v0.6.1) - 2025-01-22

### Other

- Return nicer types from crux_kv command builders
- Add command API to crux_kv
- update examples

## [0.6.0](https://github.com/redbadger/crux/compare/crux_kv-v0.5.3...crux_kv-v0.6.0) - 2025-01-21

### Other

- Integrate Commands into the Core so apps can mix and match
- Update App trait to support Command, fix all tests
- Fix clippy warnings

## [0.5.3](https://github.com/redbadger/crux/compare/crux_kv-v0.5.2...crux_kv-v0.5.3) - 2025-01-07

### Other

- update Cargo.lock dependencies

## [0.5.2](https://github.com/redbadger/crux/compare/crux_kv-v0.5.1...crux_kv-v0.5.2) - 2024-10-23

### Other

- tidy and docs update
- update http and kv tests to use new API

## [0.5.1](https://github.com/redbadger/crux/compare/crux_kv-v0.5.0...crux_kv-v0.5.1) - 2024-20-21

- Serialization of bytes can now be more efficient with [`serde_bytes`](https://github.com/serde-rs/bytes).
  see https://github.com/redbadger/crux/pull/273
- Fixes a problem building the crate for the `typegen` feature, see https://github.com/redbadger/crux/pull/277

## [0.5.0](https://github.com/redbadger/crux/compare/crux_kv-v0.4.2...crux_kv-v0.5.0) - 2024-08-13

It is no longer necessary to register types separately for this capability.
So you no longer need this in your `build.rs` file in the shared_types crate:

```rust
gen.register_type::<KeyValueResponse>()?;
gen.register_type::<KeyValueError>()?;
gen.register_type::<Value>()?;
```

### Other
- merge 0.8.1 to master

## [0.4.2](https://github.com/redbadger/crux/compare/crux_kv-v0.4.1...crux_kv-v0.4.2) - 2024-08-12

### Other
- updated the following local packages: crux_core

## [0.4.1](https://github.com/redbadger/crux/compare/crux_kv-v0.4.0...crux_kv-v0.4.1) - 2024-05-21

### Other

- Release crux_core v0.8.0
- registry now slab allocated with u32

## [0.4.0](https://github.com/redbadger/crux/compare/crux_kv-v0.3.0...crux_kv-v0.4.0) - 2024-05-17

### Breaking Changes

Introduces a `Value` enum, which can be `None` for a key that _doesn't_ exist,
or where there is no `previous_value`. See
https://github.com/redbadger/crux/pull/235.

### Other

- avoid clones of capability
- crux_kv take methods by move
- update API for crux_kv
- relax callback bounds in crux_kv

## [0.3.0](https://github.com/redbadger/crux/compare/crux_kv-v0.2.0...crux_kv-v0.3.0) - 2024-05-15

Hot on the heels of the last release, we've added a an
[operation to list keys](https://github.com/redbadger/crux/pull/232), and
[improved the app-facing API of crux_kv](https://github.com/redbadger/crux/pull/231).
The latter means that there is now a much more idiomatic Rust API for
interacting with the key-value store.

### Other

- KeyValueError::CursorNotFound, and doc comments to describe semantics
- list keys
- unwrap methods
- Simplify app-facing API of crux_kv

## [0.2.0](https://github.com/redbadger/crux/compare/crux_kv-v0.1.10...crux_kv-v0.2.0) - 2024-05-14

### Breaking Changes

This is a breaking change release. We've added `delete` and `exists` operations
and renamed `read` to `get` and `write` to `set`, amongst other renames and type
changes. We've also improved error handling and tests. See
https://github.com/redbadger/crux/pull/227 for more information. However, there
are sadly still no atomic or batch operations, which will follow in a future
release.

### Other

- deps
- Merge branch 'master' into relax-callback-bounds
- reduce nesting in kv output enums
- use enums instead of Result and Option for FFI types in crux_kv
- take owned values as cloning anyway
- move tests from integration to unit
- add exists, and update tests
- add delete to KeyValue, and move towards wasi-kv

## [0.1.10](https://github.com/redbadger/crux/compare/crux_kv-v0.1.9...crux_kv-v0.1.10) - 2024-03-24

### Other

- update Cargo.toml dependencies

## [0.1.9](https://github.com/redbadger/crux/compare/crux_kv-v0.1.8...crux_kv-v0.1.9) - 2024-02-02

### Fixed

- fix doc test deps

### Other

- Make rust fmt happy
- Export crux_macros from crux_core and change docs
- More human readable change logs

## [0.1.8](https://github.com/redbadger/crux/compare/crux_kv-v0.1.7...crux_kv-v0.1.8) - 2024-01-26

### Other

- Add async API

## [0.1.7](https://github.com/redbadger/crux/compare/crux_kv-v0.1.6...crux_kv-v0.1.7) - 2024-01-11

### Other

- update Cargo.toml dependencies

## [0.1.6](https://github.com/redbadger/crux/compare/crux_kv-v0.1.5...crux_kv-v0.1.6) - 2023-12-03

### Other

- updated the following local packages: crux_core

## [0.1.5](https://github.com/redbadger/crux/compare/crux_kv-v0.1.4...crux_kv-v0.1.5) - 2023-11-29

### Other

- update dependencies

## [0.1.4](https://github.com/redbadger/crux/compare/crux_kv-v0.1.3...crux_kv-v0.1.4) - 2023-10-25

### Other

- versions for compatibility with semver checks
- implement derive macro for Capability trait
