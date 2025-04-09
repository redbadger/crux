# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
