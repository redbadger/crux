# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.0](https://github.com/redbadger/crux/compare/crux_core-v0.13.1...crux_core-v0.14.0) - 2025-04-09

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

### Other

- clippy and doc warnings

## [0.13.1](https://github.com/redbadger/crux/compare/crux_core-v0.13.0...crux_core-v0.13.1) - 2025-04-08

Patch release to update dependency on `crux_macros` to 0.5.0

## [0.13.0](https://github.com/redbadger/crux/compare/crux_core-v0.12.2...crux_core-v0.13.0) - 2025-04-08

### Breaking Changes

`Command::notify_shell()` now returns a `NotificationBuilder` rather than a `Command` so that it can be used in sync _and_ async contexts. See [#338](https://github.com/redbadger/crux/pull/338).

### Other Changes

There is a new [`effect!`](https://docs.rs/crux_macros/latest/crux_macros/macro.effect.html) macro that improves the ergonomics of writing Crux apps significantly. Instead of creating a `Capabilities` struct and applying the `Effect` and `Export` derive macros, you can just wrap an Effect enum with the `effect!` macro and specify the unit type `()` as the `Capabilities` associated type (which will be deprecated soon) on your `App` trait implementation.

A bug was fixed in the Command executor which caused some tasks to stall under rare circumstances. See [#339](https://github.com/redbadger/crux/pull/339) for more details.

### Changes

- fix doc tests
- Adapt docs to Command API being default
- NotificationBuilder
- Fix an executor bug
- add note about capabilities deprecation in guide
- improve effect macros and docs
- typos
- register effects through register_app
- move register_types from Capability to Operation, remove caps! macro

## [0.12.2](https://github.com/redbadger/crux/compare/crux_core-v0.12.1...crux_core-v0.12.2) - 2025-03-21

Patch release, no API changes.

- Fixes a bug in the command runtime, where it was possible, under certain circumstances, to cause a panic by polling a future that had already completed.

## [0.12.1](https://github.com/redbadger/crux/compare/crux_core-v0.12.0...crux_core-v0.12.1) - 2025-03-17

This is a maintenance release (minor non-breaking changes and dependency updates).

Note: For iOS shells, we have fixed a bug in the documentation that didn't account correctly for release builds.
You may need to update your build rules accordingly.
Please see the use of `${Configuration}` in the
[documentation](https://redbadger.github.io/crux/latest_master/getting_started/iOS/with_xcodegen.html#generate-the-xcode-project-for-our-ios-app).

### Other

- rust deps
- use SystemTime in crux_time API
- update doc for Core::new()
- add track_caller to test helpers

## [0.12.0](https://github.com/redbadger/crux/compare/crux_core-v0.11.3...crux_core-v0.12.0) - 2025-02-18

***Note this is a breaking change!***

The 3 bridge functions now return a Result instead of panicking.
The error type is [`BridgeError`](https://docs.rs/crux_core/latest/crux_core/bridge/enum.BridgeError.html),
which is a custom error type that can be used to
handle errors that occur with serialization and deserialization of messages,
as well as other errors that may occur during message processing.

All the [examples](https://github.com/redbadger/crux/tree/master/examples)
have been updated to use the new error handling as well as the recent `Command` API.

### Other

- Introduce error handling in the bridge
- Add tests for various ways of abusing the bridge
- Better error messages and testing to_string on BridgeError
- update testing page in book to reflect changes
- More readable resolve
- Fix up counter
- Update dependencies

## [0.11.3](https://github.com/redbadger/crux/compare/crux_core-v0.11.2...crux_core-v0.11.3) - 2025-02-03

This is an API compatible release.

Fixes a bug in `Command::and` that could cause a stack overflow when chaining many commands together.
see [#315](https://github.com/redbadger/crux/pull/315)

### Other

- Merge pull request [#311](https://github.com/redbadger/crux/pull/311) from redbadger/command-docs
- crux_core command docs
- crux_core readme

## [0.11.2](https://github.com/redbadger/crux/compare/crux_core-v0.11.1...crux_core-v0.11.2) - 2025-01-27

### Other

- Make the SSE capability in counter example universal (usable in sync and async context)  [#310](https://github.com/redbadger/crux/pull/310)
- add `#[must_use]` to `Command`
- Make CommandContext a public type
- add from/into helpers to Command

## [0.11.1](https://github.com/redbadger/crux/compare/crux_core-v0.11.0...crux_core-v0.11.1) - 2025-01-22

### Other

- Allow .map on Request and Stream builders
- update examples

## [0.11.0](https://github.com/redbadger/crux/compare/crux_core-v0.10.1...crux_core-v0.11.0) - 2025-01-21

_This release is a breaking change._

### Changes

The `App` trait has changed to support the new `Command` API. This will break every app,
but migration is straight-forward. Please see the
[Migration Guide](https://redbadger.github.io/crux/guide/effects.html#migrating-from-previous-versions-of-crux)


## [0.10.1](https://github.com/redbadger/crux/compare/crux_core-v0.10.0...crux_core-v0.10.1) - 2025-01-07

### Other

- [drop model to avoid deadlock](https://github.com/redbadger/crux/pull/287)

## [0.10.0](https://github.com/redbadger/crux/compare/crux_core-v0.9.1...crux_core-v0.10.0) - 2024-10-23

Several additional methods to help with testing Crux apps:

- Adds a `Clone` bound on  the `Operation` trait so that we can examine the operation and still resolve
  its owning request later on — this is a breaking change.
- Adds a `take_effects` method on `Update` to allow you to take effects off the Update that match the predicate
- Adds a `take_effects_partitioned_by` method on `Update` to allow you to take effects off the Update
  that match the predicate and also the remaining effects that don't match

## [0.9.1](https://github.com/redbadger/crux/compare/crux_core-v0.9.0...crux_core-v0.9.1) - 2024-10-21

- Fixes a memory leak that affects tasks that contain futures, see https://github.com/redbadger/crux/issues/268.
  This only affects tasks that contain futures, and only when the task is dropped before the future completes.

## [0.9.0](https://github.com/redbadger/crux/compare/crux_core-v0.8.1...crux_core-v0.9.0) - 2024-08-13

### Other
- merge 0.8.1 to master

## [0.8.1](https://github.com/redbadger/crux/compare/crux_core-v0.8.0...crux_core-v0.8.1) - 2024-08-12

### Bug fixes
- Fix a crash when dropping the core caused by the drop order interacting with async channels

## [0.8.0](https://github.com/redbadger/crux/compare/crux_core-v0.7.6...crux_core-v0.8.0) - 2024-05-21

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
