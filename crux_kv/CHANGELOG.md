# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
