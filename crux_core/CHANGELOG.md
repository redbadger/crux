# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0-rc.3](https://github.com/redbadger/crux/compare/crux_core-v0.7.6...crux_core-v0.8.0-rc.3) - 2024-05-21

Release candidate for 0.8.0

Note: this is a breaking change release.

There are 2 main changes, both of which are breaking.

1. Simplify construction of capabilities, so they don't require the app type.
   The `Effect` derive macro no longer requires the name of the type that
   implements the `App` trait in situations where its name is not also `App`.
   This simplifies user code. See https://github.com/redbadger/crux/pull/241 for
   more details.

2. Requests now use `EffectId(u32)` instead of `Uuid`. This is the `id` that is
   used to identify the effect in the effect registry in order to match
   responses from the shell with their initial request. Ids are serialized as a
   plain integer of type `u32` and they can be reused as effects are resolved.
   See https://github.com/redbadger/crux/pull/238 for more details.

### Other

- update docs, comments and book
- Remove remaining mentions of the app attribute for Effect macro
- Make WithContext generic over Event, not App, enabling App types to be generic
- registry now slab allocated with u32
- add Clone impl for ComposeContext

## [0.7.6](https://github.com/redbadger/crux/compare/crux_core-v0.7.5...crux_core-v0.7.6) - 2024-05-15

### Other

- remove unused deps

## [0.7.5](https://github.com/redbadger/crux/compare/crux_core-v0.7.4...crux_core-v0.7.5) - 2024-05-14

This is a minor maintenance release, with the most interesting change being a
relaxation of the `Fn` trait bound to `FnOnce`/`FnMut` in capability event
callbacks. (see https://github.com/redbadger/crux/pull/229 for more info)

### Other

- deps
- remove Copy ound from map_event
- relax vaious func traits from Fn to FnOnce
- deps
- update all deps and dioxus examples
- typos
- Proof-read internals docs
- Convert bridges docs to anchors
- Convert runtime docs to anchors

## [0.7.4](https://github.com/redbadger/crux/compare/crux_core-v0.7.3...crux_core-v0.7.4) - 2024-03-24

### Other

- deps
- make http error a struct variant
- rust deps
- update counter example to new crux_http, WIP
- update examples to latest crux

## [0.7.3](https://github.com/redbadger/crux/compare/crux_core-v0.7.2...crux_core-v0.7.3) - 2024-02-02

### Changes

- Allow bring-your-own serializer/deserializer for the `Bridge`. This is not a
  breaking change as the existing bridge interface is the same. We have
  introduced a new `BridgeWithSerializer`, where you can plug in your own
  serialization format (e.g. JSON), but be aware that you'll have to provide
  your own serialization/deserialization shell-side code (in
  TypeScript/Swift/Kotlin).

- Re-exports `crux_macros` as `crux_core::macros` in order to help keep the
  version of macros in sync with the core. Prefer importing macros as
  `crux_core::macros`, rather than `crux_macros`

## [0.7.2](https://github.com/redbadger/crux/compare/crux_core-v0.7.1...crux_core-v0.7.2) - 2024-01-26

### Fixed

- fix clippy lints

### Other

- Introduce a Compose capability which allows composition of other capabilities
- Introduce `Never` type for capabilities that don't request effects
- Effect derive macro now allows skipping variants (to support `Never`
  operations)
- Make render capability is now Clone to support composition
- remove uuid unused wasm-bindgen feature flag

## [0.7.1](https://github.com/redbadger/crux/compare/crux_core-v0.7.0...crux_core-v0.7.1) - 2024-01-11

### Other

- update deps for Rust, Web, iOS and Android
- update examples to crux_core 0.7

## [0.7.0](https://github.com/redbadger/crux/compare/crux_core-v0.6.5...crux_core-v0.7.0) - 2023-12-03

### Fixed

- fix doc tests

### Other

- improve typegen error handling

## [0.6.5](https://github.com/redbadger/crux/compare/crux_core-v0.6.4...crux_core-v0.6.5) - 2023-11-29

### Other

- root deps
- rustfmt
- full error message

## [0.6.4](https://github.com/redbadger/crux/compare/crux_core-v0.6.3...crux_core-v0.6.4) - 2023-10-25

### Other

- update deps
- update leptos examples to remove Scope
- deps + tweaks
- avoid unnecessary coercion
- Remove existing generated java files before generating the new set
- deps
- deps
