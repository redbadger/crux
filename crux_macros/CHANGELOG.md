# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.11.0](https://github.com/redbadger/crux/compare/crux_macros-v0.10.1...crux_macros-v0.11.0) - 2026-09-06

### 🚀 Features

- **`#[derive(Operation)]` declares an operation and what the shell does with
  it.** An operation used to need a hand-written `Operation` implementation, and
  now — with `Operation::KIND` and the `crux_core::operation` marker traits —
  three things that all have to agree. The derive writes all three from one
  attribute:

  ```rust
  use crux_core::macros::Operation;

  /// Told to the shell, never answered. `Output` is `()`.
  #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
  #[operation(notify)]
  pub struct Publish(pub Vec<u8>);

  /// Answered exactly once.
  #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
  #[operation(request, output = GetResult)]
  pub struct Get {
      pub key: String,
  }

  /// Answered a sequence of times.
  #[derive(Operation, Facet, Debug, Clone, Serialize, Deserialize)]
  #[operation(stream, output = Message)]
  pub struct Subscribe;
  ```

  Each expands to an `impl Operation` with the `Output` and the matching
  `const KIND`, plus the marker trait — `crux_core::operation::Notify`,
  `Request` or `Stream` — so sending the operation with the wrong `Command`
  constructor is a compile error.

  Exactly one of `notify`, `request` and `stream` is required. `notify` takes no
  `output` (it is always `()`); `request` and `stream` require one, and it can
  be any type, including an unquoted generic one such as `Option<Vec<u8>>`.
  Generic operations and `where` clauses pass through to both implementations.

  An optional `register(A, B, ..)` names further types for type generation to
  emit alongside the operation and its output — the ones a tracer cannot reach
  from the output alone:

  ```rust
  #[operation(request, output = ValueResult, register(KeyValueError, Value))]
  pub struct Get {
      pub key: String,
  }
  ```

  It generates `register_types` and `register_types_facet` overrides, each
  behind the `typegen` and `facet_typegen` features **of the crate the derive is
  used in**, matching the gates on the trait's own methods. A crate that
  declares neither feature still compiles — the generated `impl` carries
  `#[allow(unexpected_cfgs)]`, so an undeclared feature name in the `cfg` is not
  a warning — and simply gets no overrides, which is what a crate that does no
  type generation wants. To have `register(..)` take effect, declare features of
  the matching names forwarding to `crux_core`, as `crux_kv` and `crux_time` do:

  ```toml
  [features]
  typegen = ["crux_core/typegen"]
  facet_typegen = ["crux_core/facet_typegen"]
  ```

  Structs only, of any shape: named, tuple or unit.

### ⚙️ Miscellaneous Tasks

- **`#[effect(facet_typegen)]` now records each variant's request kind and
  output type**, by calling the new `TypeRegistry::register_effect` after
  registering the effect's own types:

  ```rust
  generator
      .register_effect::<EffectFfi>()?
      .variant::<RenderOperation>("Render")?
      .variant::<HttpRequest>("Http")?
      .finish();
  ```

  Type generation uses this to emit a request-kind accessor and a typed effect
  handler API for each shell language — see the `crux_core` changelog.
  `#[effect(typegen)]` — the legacy serde path — is unchanged.

## [0.10.1](https://github.com/redbadger/crux/compare/crux_macros-v0.10.0...crux_macros-v0.10.1) - 2026-08-06

### 🐛 Bug Fixes

- **A generic operation type now registers correctly under typegen.** `#[effect]`
  interpolated each variant's operation type into *expression* position when
  generating `register_types`:

  ```rust
  #operation::register_types(generator)?;
  ```

  That parses for a bare path, but for a generic operation it does not —
  `Navigate<Route>::register_types(generator)` parses as a chain of comparisons, so
  an app declaring one got a syntax error pointing into macro output rather than at
  its own code. The operation is now interpolated as a qualified path,
  `<#operation>::register_types(generator)?`, which parses for any type. Both typegen
  paths were affected, serde and facet.

  Released as a patch deliberately. Both `crux_core` 0.19.0 and 0.20.0 require
  `crux_macros ^0.10.0`, so 0.10.1 reaches everyone on either one, including people who
  are not ready to move to `crux_core` 0.20.0. A 0.11.0 would reach neither without a
  further `crux_core` release.

### ⚙️ Miscellaneous Tasks

- No other public API changes, and nothing consumer-visible in the manifest: this crate's
  only workspace dependencies (`facet`, `facet-generate-attrs`) are dev-dependencies, so
  it is unaffected by the `facet_generate` bump that `crux_core` 0.20.0 carries.
- `crux_platform` has been removed from Crux and is deprecated — see the README on
  [crates.io](https://crates.io/crates/crux_platform). Its source is gone from this
  repository, and the macro test fixtures and README example no longer mention it.
- Dependency updates, including `syn` 2 -> 3 and `darling` 0.23 -> 0.24.
- `categories` added to the crate metadata for crates.io.

## [0.10.0](https://github.com/redbadger/crux/compare/crux_macros-v0.9.0...crux_macros-v0.10.0) - 2026-05-31

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.19.0. No public API changes.
- Internal clippy nursery improvements.

## [0.9.0](https://github.com/redbadger/crux/compare/crux_macros-v0.8.0...crux_macros-v0.9.0) - 2026-05-07

### 🚀 Features

- **Fluent effect test assertions**: The `#[effect]` macro now generates an `<Effect>TestExt`
  trait on `Command<Effect, Event>` with chainable, per-variant assertion and resolution methods:
  `expect_<effect>`, `expect_<effect>_with`, `expect_only_<effect>`, `expect_only_<effect>_with`,
  `resolve_<effect>`, and `then_event`. These allow integration tests to be written as a
  readable chain of assertions and responses. The helpers are compiled in only when the `testing`
  feature is enabled on `crux_core`.

- **Improved assertion error messages**: Effect and event assertion failures now report how many
  effects and events remain, making failing tests easier to diagnose.

- **`Requests` type registered for facet typegen**: The generated typegen code now also registers
  `crux_core::bridge::Requests<EffectFfi>` alongside `Request<EffectFfi>`, ensuring the batch
  request wrapper type is included in generated bindings.

### ⚙️ Miscellaneous Tasks

- Updated to `facet_generate` 0.17.
- Align with `crux_core` 0.18.0.
- Dependency updates.

## [0.8.0](https://github.com/redbadger/crux/compare/crux_macros-v0.7.0...crux_macros-v0.8.0) - 2026-03-20

### 🚀 Features

**This is a breaking release.**

- **Removed Capability Support**: Removed all capability-related derive macros and machinery as part of the migration to the Command API.
- **Enhanced Effect Macros**: Improved `#[effect]` attribute macro with better support for type generation and Command integration.
- **Bridge Integration**: Updated macros to work with the new unified Bridge API in `crux_core`.

### 🐛 Bug Fixes

- Transfer facet attributes to effect FFI enum, fixing an issue where attributes were not being copied.

### ⚙️ Miscellaneous Tasks

- Update to `facet_generate` 0.15, `facet` 0.31, and other Rust dependencies.
- Remove deprecated capability derive macros.
- Align with Command API requirements.
- Update Rust dependencies and align with `crux_core` 0.17.0.

## [0.7.0](https://github.com/redbadger/crux/compare/crux_macros-v0.6.1...crux_macros-v0.7.0) - 2025-07-31

### 🚀 Features

- *(crux_core)* Facet-based typegen frontend
- *(crux_macros)* Be explicit in effect macro about typegen kind
- *(crux_core)* Facet typegen with module support

### 🐛 Bug Fixes

- *(crux_cli)* Remove need for env::set_var as unsafe in 2024 edition
- *(crux_core)* Pass facet-typegen feature through core to macros
- *(crux)* Fix tests for all feature combinations

### ⚙️ Miscellaneous Tasks

- *(crux)* Format fixes
- Use facet-generate 0.3.0
- *(crux_macros)* Fix features
- *(crux_core)* Use facet_generate v0.4

## [0.6.1](https://github.com/redbadger/crux/compare/crux_macros-v0.6.0...crux_macros-v0.6.1) - 2025-05-27

Minor changes, not breaking.

### Changes

- Automatically implement `TryFrom` to fallibly downcast Effects into specific Requests
- Fix bug in `#[effect(typegen)]` macro that didn't recognise enum name
- remove need for nightly on new typegen
- uniffi 0.29.2 and other deps
- clippy pedantic

## [0.6.0](https://github.com/redbadger/crux/compare/crux_macros-v0.5.0...crux_macros-v0.6.0) - 2025-04-09

### Breaking Change

The `effect!` function macro has been replaced with an `#[effect]` attribute proc macro.

It also expects you to opt into foreign type generation using `#[effect(typegen)]` attribute on your Effect enum.

This is to allow the macro to be used in applications that either have a Rust shell, don't use the builtin typegen _or_  use the new typegen (from #217) by using `#[effect]` (without any arguments).

e.g. with typegen:

```rust
#[effect(typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest)
}
```

or, without typegen:

```rust
#[effect]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    ServerSentEvents(SseRequest)
}
```

### Added

- *(macros)* [**breaking**] effect! macro is now #[effect] attribute macro

## [0.5.0](https://github.com/redbadger/crux/compare/crux_macros-v0.4.5...crux_macros-v0.5.0) - 2025-04-08

### Breaking Change

Note that this release replaces v0.4.5 (which has been yanked for a breaking change in typegen for existing code).

There is a new [`effect!`](https://docs.rs/crux_macros/latest/crux_macros/macro.effect.html) macro that improves the ergonomics of writing Crux apps significantly. Instead of creating a `Capabilities` struct and applying the `Effect` and `Export` derive macros, you can just wrap an Effect enum with the `effect!` macro and specify the unit type `()` as the `Capabilities` associated type (which will be deprecated soon) on your `App` trait implementation.

## [0.4.5](https://github.com/redbadger/crux/compare/crux_macros-v0.4.4...crux_macros-v0.4.5) - 2025-03-17

This is a maintenance release (minor non-breaking changes and dependency updates).

We have also added the `#[track_caller]` attribute to various test helper functions,
so that we can get a more useful line number for test failures.

### Fixed

- fix tests

### Other

- updated rust dependencies
- add track_caller to test helpers

## [0.4.4](https://github.com/redbadger/crux/compare/crux_macros-v0.4.3...crux_macros-v0.4.4) - 2025-01-22

### Other

- update dependencies
- update examples

## [0.4.3](https://github.com/redbadger/crux/compare/crux_macros-v0.4.2...crux_macros-v0.4.3) - 2025-01-21

### Other

- Fix doctests
- update Effect derive macro for From<request<Op>>
- Fix clippy warnings

## [0.4.2](https://github.com/redbadger/crux/compare/crux_macros-v0.4.1...crux_macros-v0.4.2) - 2025-01-07

### Other

- deps

## [0.4.1](https://github.com/redbadger/crux/compare/crux_macros-v0.4.0...crux_macros-v0.4.1) - 2024-20-21

- no changes, just updated dependencies

## [0.4.0](https://github.com/redbadger/crux/compare/crux_macros-v0.3.10...crux_macros-v0.4.0) - 2024-08-13

### Other
- capabilities can now do their own type registration, so it's no longer
  necessary to register types in the `build.rs` in the `shared_types` crate.
- crux_http and crux_kv now register their additional types
- override typegen for Compose capability to panic with msg
- Making #[effect(skip)] skip fields when serializing
- rust deps

## [0.3.10](https://github.com/redbadger/crux/compare/crux_macros-v0.3.9...crux_macros-v0.3.10) - 2024-05-21

### Other

- update docs, comments and book
- Remove remaining mentions of the app attribute for Effect macro
- Make WithContext generic over Event, not App, enabling App types to be generic

## [0.3.9](https://github.com/redbadger/crux/compare/crux_macros-v0.3.8...crux_macros-v0.3.9) - 2024-05-14

This is a minor maintenance release, with the most interesting change being a fix for the export derive macro to work with renamed `Effect` types. (see https://github.com/redbadger/crux/pull/221 for more info)

### Other

- deps
- Merge branch 'master' into relax-callback-bounds
- add test
- allow export derive macro to name effect
- deps
- update all deps and dioxus examples

## [0.3.8](https://github.com/redbadger/crux/compare/crux_macros-v0.3.7...crux_macros-v0.3.8) - 2024-03-24

### Other

- deps
- make http error a struct variant
- rust deps
- update counter example to new crux_http, WIP
- update examples to latest crux

## [0.3.7](https://github.com/redbadger/crux/compare/crux_macros-v0.3.6...crux_macros-v0.3.7) - 2024-02-02

### Changed

- Only works with `crux_core` 0.7.3 or later.
- You should now import the macros from `crux_core::macros` rather than from
  this crate directly. This should avoid compatibility issues between the core
  and the macros in the future.

## [0.3.6](https://github.com/redbadger/crux/compare/crux_macros-v0.3.5...crux_macros-v0.3.6) - 2024-01-26

### Other

- darling default
- unimplemented rather than todo
- docs for effect macro
- effect derive macro allows skipping variants (e.g. for Never operations)
- update Rust deps

## [0.3.5](https://github.com/redbadger/crux/compare/crux_macros-v0.3.4...crux_macros-v0.3.5) - 2024-01-11

### Other

- update deps for Rust, Web, iOS and Android

## [0.3.4](https://github.com/redbadger/crux/compare/crux_macros-v0.3.3...crux_macros-v0.3.4) - 2023-11-29

### Other

- root deps

## [0.3.3](https://github.com/redbadger/crux/compare/crux_macros-v0.3.2...crux_macros-v0.3.3) - 2023-10-25

### Other

- update deps
- update leptos examples to remove Scope
- deps + tweaks
- deps
- deps
- deps
- deps, http 0.4.1, time 0.1.4
- update deps, iOS and Android examples
