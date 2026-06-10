# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 💥 Breaking Changes

**`ResponseAsync` renamed to `RawResponse`.**

The type that flows through the middleware chain has been renamed from
`ResponseAsync` to `RawResponse`. The old name was misleading: the struct
holds a plain `(StatusCode, HeaderMap, Vec<u8>)` and nothing about it is
asynchronous. `RawResponse` accurately describes its role — the unvalidated
response from the shell before the 4xx/5xx error check in `Response::new()`.

Update all imports and type annotations:

```rust
// Before
use crux_http::ResponseAsync;
async fn handle(…) -> Result<ResponseAsync> { … }

// After
use crux_http::RawResponse;
async fn handle(…) -> Result<RawResponse> { … }
```

**`RawResponse` body-reading methods are now synchronous.**

`body_bytes()`, `body_string()`, `body_json()`, and `body_form()` on
`RawResponse` no longer return a future — they return `Result<T>` directly.
Remove `.await` at every call site. All body data in `crux_http` is
already in memory, so the `async` was superfluous.

```rust
// Before
let bytes = res.body_bytes().await?;
// After
let bytes = res.body_bytes()?;
```

**`Request::set_content_type` now takes `&mime::Mime` instead of `mime::Mime`.**

The method no longer takes ownership of the MIME type. Pass a reference:

```rust
// Before
req.set_content_type(mime::APPLICATION_JSON);
// After
req.set_content_type(&mime::APPLICATION_JSON);
```

The builder-level `.content_type(…)` methods on `RequestBuilder` are
unaffected — they still accept any `impl Into<Mime>` by value.

---

**`crux_http::http` now re-exports the real [`http`](https://docs.rs/http) crate (v1.4), not `http-types`.**

This is the main breaking change. `crux_http::http` used to be a re-export of the
`http-types-red-badger-temporary-fork` crate. It is now always the upstream `http` crate.
The most common impacts:

| Scenario | Action |
| --- | --- |
| App only uses `crux_http::{Http, RequestBuilder, Response, …}` and `crux_http::Method` | Likely **compiles unchanged**. `Method` is now `http::Method`; its API is compatible for common uses (`Method::GET`, `Method::POST`, …). |
| Code references `Method::Get`, `Method::Post`, … (UpperCamelCase variants) | Rename to `Method::GET`, `Method::POST`, … (associated constants on `http::Method`). |
| `crux_http::http::StatusCode::Unauthorized` etc. | Rename to `http::StatusCode::UNAUTHORIZED` etc. `HttpError::Http { code, .. }` now stores the code as a plain `u16`; compare with `401u16` or `StatusCode::UNAUTHORIZED.as_u16()`. |
| Imports `crux_http::http::mime::HTML` etc. | Use `crux_http::mime::TEXT_HTML` (or any constant from the `mime` crate, now re-exported as `crux_http::mime`). |
| Imports `crux_http::http::Body` / `Headers` / `Version` | Use `crux_http::Body` (new crux-owned type) or `http::HeaderMap` / `http::Version` directly. |
| Used the `http-compat` feature | The feature is **removed**. Native lossless conversions (`From`/`TryFrom`) between `crux_http` types and `http::Request<Body>` / `http::Response<Body>` are now provided unconditionally — no feature flag required. |
| Has code that builds or consumes `http_types::Request`/`Response` | Enable the new **`http-types`** feature; it provides `From`/`Into` conversion impls between `crux_http` and `http_types` types. |
| Relied on streaming `http_types::Body` / `AsyncRead` on `ResponseAsync` | The streaming body model is not carried over. The type is now called `RawResponse`; refactor streaming to use the `Chunk`/`Done` capability pattern. |

### 🚀 Features

- **New `crux_http::Body` type** — a simple, synchronous, in-memory request body with
  an optional MIME type.  Replaces the async `http_types::Body`.  Provides `Into<Body>`
  conversions from `String`, `&str`, `Vec<u8>`, `&[u8]`, and `serde_json::Value`, plus
  `into_bytes()`, `mime()`, `len()`, and `is_empty()`.
- **`crux_http::mime` re-export** — the `mime` crate (v0.3) is now re-exported directly
  as `crux_http::mime`, giving access to constants like `mime::APPLICATION_JSON` and
  `mime::TEXT_HTML` without needing a separate dependency.
- **Native `http` conversions** — `From<http::Request<Body>> for crux_http::Request`,
  `From<crux_http::Request> for http::Request<Body>`, and
  `TryFrom<crux_http::Response<Body>> for http::Response<Body>` are available out of the
  box without any feature flag.
- **`http-types` compat feature** — add `crux_http = { features = ["http-types"] }` to
  your `Cargo.toml` to get `From`/`Into` impls between `crux_http` types and the legacy
  `http_types` types, as a bridge while migrating.
- **`Response::header_all(name)`** — returns all values for a given header name (via
  `http::HeaderMap::get_all`), useful when a server sends multiple values for the same
  header.

### ⚙️ Miscellaneous Tasks

- `http_types` (the temporary fork) is no longer a default dependency; it is pulled in
  only when the `http-types` feature is enabled, reducing the default dependency footprint.
- `into_protocol_request` is now a synchronous function (was `async`); the previous
  `await` on `http_types::Body::into_bytes()` is gone.
- `ResponseAsync` renamed to `RawResponse` throughout.

## [0.18.0](https://github.com/redbadger/crux/compare/crux_http-v0.17.0...crux_http-v0.18.0) - 2026-05-31

### 🚀 Features

- **Improved testing ergonomics**: `FakeShell::provide_response` and
  `FakeShell::take_requests_received` now take `&self` instead of `&mut self`,
  making test setup less fiddly.

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.19.0.
- Internal clippy nursery improvements.
- Dependency updates (`web-sys` 0.3.99).

## [0.17.0](https://github.com/redbadger/crux/compare/crux_http-v0.16.0...crux_http-v0.17.0) - 2026-05-07

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.18.0. No public API changes.
- Dependency updates.

## [0.16.0](https://github.com/redbadger/crux/compare/crux_http-v0.15.0...crux_http-v0.16.0) - 2026-03-20

### 🚀 Features

**This is a breaking release.**

- **Command API Support**: Updated to work with the new Command API, removing dependency on the deprecated Capability trait. (This should be non-breaking from the Command API's perspective - the command modules stay around and re-export the code which moved back into the root).
- **Enhanced Testing**: Improved testing support with new command-based test helpers.
- **Breaking**: Updated to `serde_qs` v1.0, which relaxes querystring encoding to be more RFC 3986 compliant. Special characters like `;/?:@$,-.!~*'()` are no longer percent-encoded, and spaces are encoded as `+`. This may change the URLs generated by `.query()` calls.
- **Breaking**: Reorder `HttpError` variants to preserve discriminants across FFI boundary. The external error variants (`Url`, `Io`, `Timeout`) now come first to maintain stable discriminants for shell bindings.
- **Breaking**: Remove facet annotations from `http_types` references, keeping them only for our own protocol types that cross the FFI boundary.

### 🐛 Bug Fixes

- Remove (but then revert) `http_types` fork dependency — it _is_ actually still needed for `traceid` (replace u128 with u64) in order to be compatible with the emscripten target. However we no longer require it to have types annotated with the Facet derive, and we'll remove them from the fork in a future release.

### ⚙️ Miscellaneous Tasks

- Update to `facet_generate` 0.15 and `facet` 0.31.
- Migrate internal implementation from Capability to Command API.
- Update documentation and examples for Command-based usage.
- Update Rust dependencies.
- Migrate tests to use `insta` snapshot assertions.

## [0.15.0](https://github.com/redbadger/crux/compare/crux_http-v0.14.0...crux_http-v0.15.0) - 2025-07-31

### 🚀 Features

- *(crux_core)* Facet-based typegen frontend

### 🚜 Refactor

- *(crux_http)* Drop `unsafe` block

### ⚙️ Miscellaneous Tasks

- *(crux)* Format fixes

## [0.14.0](https://github.com/redbadger/crux/compare/crux_http-v0.13.0...crux_http-v0.14.0) - 2025-05-27

This is a minor bump because of breaking changes in `crux_core`

### Other

- add `.query()` method to `protocol::HttpRequest`
- clippy pedantic

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
