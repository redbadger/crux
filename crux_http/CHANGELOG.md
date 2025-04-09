# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.0](https://github.com/redbadger/crux/compare/crux_http-v0.12.0...crux_http-v0.13.0) - 2025-04-09

### Other

- updated the following local packages: crux_core

## [0.12.0](https://github.com/redbadger/crux/compare/crux_http-v0.11.10...crux_http-v0.12.0) - 2025-04-09
Replaces yanked 0.11.10 as that broke typegen on older versions of crux_core.

## [0.11.10](https://github.com/redbadger/crux/compare/crux_http-v0.11.9...crux_http-v0.11.10) - 2025-04-08

Updated to use version 0.13 of [`crux_core`](https://crates.io/crates/crux_core).

### Other

- rust deps
- NotificationBuilder
- move register_types from Capability to Operation, remove caps! macro
- doc comments

## [0.11.9](https://github.com/redbadger/crux/compare/crux_http-v0.11.8...crux_http-v0.11.9) - 2025-03-21

Patch release, no API changes.

### Other

- updated the following local packages: crux_core

## [0.11.8](https://github.com/redbadger/crux/compare/crux_http-v0.11.7...crux_http-v0.11.8) - 2025-03-17

This is a maintenance release (minor non-breaking changes and dependency updates).

### Other

- rust deps

## [0.11.7](https://github.com/redbadger/crux/compare/crux_http-v0.11.6...crux_http-v0.11.7) - 2025-02-18

### Other

- Updates internal tests for error handling when resolving requests

## [0.11.6](https://github.com/redbadger/crux/compare/crux_http-v0.11.5...crux_http-v0.11.6) - 2025-02-03

### Other

- crux_core command docs wip

## [0.11.5](https://github.com/redbadger/crux/compare/crux_http-v0.11.4...crux_http-v0.11.5) - 2025-01-27

- crux_http: change an unwrap to an expect [#307](https://github.com/redbadger/crux/pull/307)

## [0.11.4](https://github.com/redbadger/crux/compare/crux_http-v0.11.3...crux_http-v0.11.4) - 2025-01-24

This release adds a new [`command`](https://docs.rs/crux_http/latest/crux_http/command/index.html)
module with support for generating commands for HTTP requests.

This is to support the new Crux API. Please see the
[Migration Guide](https://redbadger.github.io/crux/guide/effects.html#migrating-from-previous-versions-of-crux)

## [0.11.3](https://github.com/redbadger/crux/compare/crux_http-v0.11.2...crux_http-v0.11.3) - 2025-01-22

### Other

- fix API docs

## [0.11.2](https://github.com/redbadger/crux/compare/crux_http-v0.11.0...crux_http-v0.11.2) - 2025-01-22

### Other

- `http` crate compatibility (conversions for request and response)
- update examples

## [0.11.0](https://github.com/redbadger/crux/compare/crux_http-v0.10.4...crux_http-v0.11.0) - 2025-01-21

### Fixed

- fix failing cargo check

### Other

- Update http test to use command for rendering
- Integrate Commands into the Core so apps can mix and match
- Update App trait to support Command, fix all tests
- add body_form to http request builder

## [0.10.4](https://github.com/redbadger/crux/compare/crux_http-v0.10.3...crux_http-v0.10.4) - 2025-01-07

### Other

- update Cargo.lock dependencies

## [0.10.3](https://github.com/redbadger/crux/compare/crux_http-v0.10.2...crux_http-v0.10.3) - 2024-10-23

### Other

- tidy and docs update
- update http and kv tests to use new API

## [0.10.2](https://github.com/redbadger/crux/compare/crux_http-v0.10.1...crux_http-v0.10.2) - 2024-20-21

- Fixes a problem building the crate for the `typegen` feature, see https://github.com/redbadger/crux/pull/277.

## [0.10.1](https://github.com/redbadger/crux/compare/crux_http-v0.10.0...crux_http-v0.10.1) - 2024-09-30

Serialization of bytes can now be more efficient with [`serde_bytes`](https://github.com/serde-rs/bytes).
This should be a non-breaking change.

## [0.10.0](https://github.com/redbadger/crux/compare/crux_http-v0.9.3...crux_http-v0.10.0) - 2024-08-13

It is no longer necessary to register types separately for this capability.
So you no longer need this in your `build.rs` file in the `shared_types` crate:

```rust
gen.register_type::<HttpError>()?;
```

### Other
- merge 0.8.1 to master

## [0.9.3](https://github.com/redbadger/crux/compare/crux_http-v0.9.2...crux_http-v0.9.3) - 2024-08-12

### Other
- updated the following local packages: crux_core

## [0.9.2](https://github.com/redbadger/crux/compare/crux_http-v0.9.1...crux_http-v0.9.2) - 2024-05-21

### Other

- Release crux_core v0.8.0

## [0.9.1](https://github.com/redbadger/crux/compare/crux_http-v0.9.0...crux_http-v0.9.1) - 2024-05-14

Minor maintenance release

### Other

- deps
- remove Copy ound from map_event
- deps
- update all deps and dioxus examples

## [0.8.1](https://github.com/redbadger/crux/compare/crux_http-v0.8.0...crux_http-v0.8.1) - 2024-03-24

### Other

- deps
- make http error a struct variant
- update counter example to new crux_http, WIP

## [0.8.0](https://github.com/redbadger/crux/compare/crux_http-v0.7.0...crux_http-v0.8.0) - 2024-02-26

### Breaking Changes

- Fixes a type generation problem with the newly exposed `HttpResult` and
  `HttpError`
- **Requires explicit tracing of crux_http::HttpError when using typegen**

## [0.7.0](https://github.com/redbadger/crux/compare/crux_http-v0.6.0...crux_http-v0.7.0) - 2024-02-21

### Breaking changes

- **The protocol between shell and core has changed. Core now expects a
  `HttpResult` rather than a `HttpResponse`**

## [0.6.0](https://github.com/redbadger/crux/compare/crux_http-v0.5.1...crux_http-v0.6.0) - 2024-02-06

### Breaking changes

- **When using the (sync) APIs which return response in an Event, HTTP responses
  with status code in the 4xx and 5xx range are now considered an error.**
- Better Error type with more detail, allowing apps to handle HTTP errors with
  more specificity
- When handling error such error responses, `crux_http` won't attempt to
  deserialize the body into the expected type, which would almost certainly fail
  and obscure the actual cause of the error with a serde error
- Http errors now contain the body bytes if present

## [0.5.1](https://github.com/redbadger/crux/compare/crux_http-v0.5.0...crux_http-v0.5.1) - 2024-02-02

### Changed

- Depends on a fork of `http_types` that will compile for the
  `wasm32-unknown-emscripten` target.

## [0.5.0](https://github.com/redbadger/crux/compare/crux_http-v0.4.6...crux_http-v0.5.0) - 2024-01-30

### Fixed

- fix doc test deps

### Other

- remove http_types default features from crux_http
- More human readable change logs

## [0.4.6](https://github.com/redbadger/crux/compare/crux_http-v0.4.5...crux_http-v0.4.6) - 2024-01-26

### Fixed

- fix clippy lints

### Other

- Add async API support

## [0.4.5](https://github.com/redbadger/crux/compare/crux_http-v0.4.4...crux_http-v0.4.5) - 2024-01-11

### Other

- update deps for Rust, Web, iOS and Android

## [0.4.4](https://github.com/redbadger/crux/compare/crux_http-v0.4.3...crux_http-v0.4.4) - 2023-12-03

### Other

- updated the following local packages: crux_core, crux_core

## [0.4.3](https://github.com/redbadger/crux/compare/crux_http-v0.4.2...crux_http-v0.4.3) - 2023-11-29

### Other

- root deps

## [0.4.2](https://github.com/redbadger/crux/compare/crux_http-v0.4.1...crux_http-v0.4.2) - 2023-10-25

### Other

- versions for compatibility with semver checks
- update deps
- deps + tweaks
- deps
- deps
- deps
- capability doc tests
- deps
