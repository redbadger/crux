# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
