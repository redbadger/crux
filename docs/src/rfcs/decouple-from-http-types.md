# RFC: Decouple `crux_http` from `http-types`

```admonish
This RFC is a **proposal** and has not yet been adopted. We are looking for
feedback on the overall direction and the open questions, rather than the exact
public API details.
```

Related issues:

- [#285 — Migrate from `crux_http` to standard `http` crate types](https://github.com/redbadger/crux/issues/285) (milestone 0.19)
- [#357 — Investigate gradual migration path of `crux_http` to `http`](https://github.com/redbadger/crux/issues/357)
- [#195 — emscripten compatibility (reason for the `http-types` fork)](https://github.com/redbadger/crux/issues/195)

## Summary

`crux_http` is currently built on top of [`http-types`](https://docs.rs/http-types)
(specifically our fork, `http-types-red-badger-temporary-fork`). `http-types` is
no longer actively maintained, and the Rust ecosystem has converged on the
[`http`](https://docs.rs/http) crate (the "hyperium" types) as the lingua franca
for HTTP request/response/header/method/status types.

This document proposes:

1. Moving the `http-types` dependency, and all code that touches it, behind an
   opt-in `http-types` Cargo feature.
2. Making the **default** build of `crux_http` use the `http` crate's types
   natively.
3. Replacing the existing `http-compat` feature (which only provides
   `From`/`TryInto` conversions to/from `http` types) with first-class, native
   support for `http` types.

The goal, as framed in #357, is that **existing consumers can enable the
`http-types` feature and continue as before, while new users get `http`
compatibility out of the box.**

## Why?

- **`http-types` is unmaintained.** We already carry a temporary fork
  (`http-types-red-badger-temporary-fork`) purely to work around an emscripten
  build issue (#195). Carrying a fork of an abandoned crate is a long-term
  liability.
- **The community standard is `http`.** Consumers increasingly use crates that
  interoperate via `http` types (e.g. `reqwest`, `axum`, `tower-http`, `hyper`).
  Issue #285 reports concrete friction: users have to hand-convert between
  `crux_http`'s `http-types`-based types and the `http` types their other
  dependencies expect.
- **The `http-compat` feature is a partial, lossy bridge.** It only offers
  one-directional conversions (`http::Request<B> -> crux_http::Request` and
  `crux_http::Response<Body> -> http::Response<Body>`) and silently drops
  information (e.g. unsupported HTTP versions, non-UTF-8 headers via `unwrap`).
  Native support is strictly better.

## Background: what `http-types` gives us vs. what `http` gives us

This migration is **not** a drop-in dependency swap. `http-types` is a rich,
batteries-included HTTP model, whereas `http` is deliberately a minimal set of
*type definitions* with no I/O, no body model, and no MIME handling. Knowing the
gap precisely is the crux of this design.

### `http-types` API surface currently used by `crux_http`

Gathered from across the crate (`lib.rs`, `request.rs`, `request_builder.rs`,
`response/*.rs`, `config.rs`, `client.rs`, `command.rs`, `error.rs`,
`middleware/redirect.rs`, `protocol.rs`):

| `http-types` item | Where used | `http`-crate equivalent |
| --- | --- | --- |
| `Method` | everywhere; re-exported as `crux_http::Method` | `http::Method` (direct) |
| `StatusCode` | `Response`, `ResponseAsync`, `HttpError::Http`, redirect | `http::StatusCode` (direct; but see "opaque" note) |
| `Version` | `Response`, `ResponseAsync` | `http::Version` (direct; opaque consts, not an enum) |
| `Url` | re-exported (actually from the `url` crate already) | keep `url::Url`; `http` uses `Uri` |
| `Headers`, `HeaderName`, `HeaderValue`, `HeaderValues`, `ToHeaderValues` | request/response headers, `header_serde` | `http::HeaderMap` / `HeaderName` / `HeaderValue` (different semantics: multi-value model, byte-based values) |
| `headers::{CONTENT_TYPE, LOCATION}` and iterators (`Iter`, `IterMut`, `Names`, `IntoIter`) | header access, redirect | `http::header::*` constants + `HeaderMap` iterators (different semantics) |
| `Body` | `Request`/`ResponseAsync` body, `body(impl Into<Body>)`, `set_body`, `take_body`, `into_bytes`, `AsyncRead` | no equivalent — `http` bodies are a generic `B` |
| `Mime`, `mime::{HTML, JSON, ...}` | `content_type`, re-exported as `crux_http::http::mime` | no equivalent — use the [`mime`](https://docs.rs/mime) crate |
| `Request` / `Response` (concrete, body-owning) | the structs `crux_http::Request`/`ResponseAsync` wrap | `http::Request<B>` / `http::Response<B>` (different semantics: generic over body, no I/O, `Uri` not `Url`) |
| `Error` | `response/decode.rs`, `From<http_types::Error> for HttpError` | no equivalent — `http` has small per-type errors only |
| `convert::DeserializeOwned` | `command.rs`, `request_builder.rs`, `expect.rs` | use `serde::de::DeserializeOwned` directly (direct) |
| `query()` / `set_query()` | `Request` | no equivalent — reimplement with `serde_qs` (already a dependency) |

### The three real gaps

1. **`Body`.** `http-types::Body` is an async, streaming body with
   `into_bytes()`, `AsyncRead`, MIME tracking, and `Into<Body>` impls for
   `String`, `&str`, `Vec<u8>`, `serde_json::Value`, files, readers, etc. In
   `http`, the body is just the generic parameter `B` — there is no body type at
   all. As #357 notes, we will need **our own `Body` type** (or to settle on
   `Vec<u8>` / `bytes::Bytes`).
2. **`Mime`.** Used by `content_type()` and the `body_*` helpers to set
   `Content-Type`. The standalone [`mime`](https://docs.rs/mime) crate is the
   natural replacement (it is even what `http-types` re-exported).
3. **`Error`.** `http-types::Error` is a status-carrying error type. We already
   have our own `HttpError`; we can drop the `From<http_types::Error>` bridge and
   the `http_types::Error` use in `decode.rs`.

### Key insight: the wire protocol is already `http-types`-free

The shell-facing protocol types in `protocol.rs` — `HttpRequest`, `HttpResponse`,
`HttpHeader` — already model the body as `Vec<u8>` and headers as
`Vec<{name, value}>` strings. They do **not** depend on `http-types` at all. All
the `http-types` `Body`/`Mime`/`AsyncRead` machinery is *internal scaffolding*
that is ultimately collapsed to bytes in
`ProtocolRequestBuilder::into_protocol_request` and rebuilt from bytes in
`From<HttpResponse> for ResponseAsync`.

This matters because it means **the async body model is not load-bearing for the
core effect contract.** It exists mainly to power the ergonomic builder API and
the async middleware story. That gives us latitude to replace `Body` with
something much simpler in the default path.

## Naming collision warning (breaking change to be managed)

`crux_http` currently does:

```rust,ignore
pub use http_types as http;        // crux_http::http  ==  http-types (!)
pub use http_types::Method;
pub use url::Url;
```

So today `crux_http::http` resolves to **`http-types`**, not the `http` crate.
Code in the wild uses `crux_http::http::mime::HTML`, `crux_http::http::Method`,
`crux_http::http::{Url, Version}`, etc.

Because features must be **additive** — enabling a feature should only add to a
crate's API, never change or remove things — `crux_http::http` will always
re-export the real `http` crate, regardless of whether `http-types` is enabled.
The `http-types` feature adds interop on top; it does not alter the default
representation. Existing code that imports `crux_http::http::Body`,
`crux_http::http::mime::HTML`, etc. will need to update its import paths (this is
a breaking release), but the migration is mechanical and the target is clear.

## Foreign type generation (Swift, Kotlin, TypeScript)

A natural worry is that `http::Request<B>` / `http::Response<B>` are **generic**,
and the foreign types we emit with `facet_generate` for Swift, Kotlin and
TypeScript cannot be generic. This does **not** affect the design, because none
of the `http` (or `http-types`) types ever reach the type generator.

The only types registered for generation are the non-generic wire-protocol types
in `protocol.rs` (plus `HttpError`), via `Operation::register_types` and Facet
reflection:

- `HttpRequest` — `method: String`, `url: String`, `headers: Vec<HttpHeader>`, `body: Vec<u8>`
- `HttpResponse` — `status: u16`, `headers: Vec<HttpHeader>`, `body: Vec<u8>`
- `HttpHeader` — `name: String`, `value: String`
- `HttpResult` — `enum { Ok(HttpResponse), Err(HttpError) }`
- `HttpError` — the `Http { code, .. }` variant is `#[facet(skip)]` + `#[serde(skip)]`, so it is never emitted to foreign types

All of these are concrete and contain only primitives, owned strings, byte
vectors, and each other. The generic `http::Request<B>`/`http::Response<B>`, the
non-`Facet` `http::Method`/`StatusCode`/`HeaderMap`, and the crux façade types
(`crux_http::Request`, `Response<Body>`, `ResponseAsync`, and the proposed
crux-owned `Body`) are **core-side only** — they are never registered, never
serialized across the bridge, and never type-generated.

This is the same observation as "the wire protocol is already `http-types`-free",
seen from the codegen angle: because bodies are already collapsed to `Vec<u8>`
and headers to `Vec<{String, String}>` at the protocol boundary, swapping the
core-side representation is invisible to foreign type generation, and the
generated Swift/Kotlin/TypeScript types are byte-for-byte unchanged.

The one detail to keep tidy is `HttpError::Http.code` (see
"Resolved decisions"): it is `http_types::StatusCode` today, carried with
`#[facet(opaque)]` because it is not a `Facet` type. We will store it as a plain
`u16` and drop the `#[facet(opaque)]` attribute. The variant is skipped from
generation regardless, so foreign output is unaffected.

## Goals

- Default build depends on `http`, not `http-types`.
- Foreign type generation output (Swift/Kotlin/TypeScript) is unchanged, because
  only the non-generic wire-protocol types are ever generated.
- `http-types` becomes an opt-in feature that restores today's behaviour and
  public surface as closely as practical.
- Remove the `http-compat` feature; its conversions become unnecessary because
  the default types *are* `http` types.
- Drop the `http-types-red-badger-temporary-fork` dependency from the default
  build (and with it, ideally, the emscripten workaround burden — #195).
- Keep the shell-facing protocol (`HttpRequest`/`HttpResponse`/`HttpResult`) and
  the FFI/typegen output **unchanged**. This migration must be invisible across
  the bridge.
- Keep `crux_http::Method`, `crux_http::Url`, `Request`, `Response`,
  `RequestBuilder`, the `command::Http` API, and middleware working in both
  configurations.

## Non-goals

- Changing the wire protocol or typegen output.
- Adding real network I/O to `crux_http` (it remains a request-describing
  capability; the shell performs the request).
- A streaming body model on the default path. Bodies are bounded `Vec<u8>` today
  at the protocol boundary, and we will keep that.
- Perfectly preserving every `http-types`-specific method signature on the
  default path. Source-level breakage is expected for users who do not opt into
  the `http-types` feature; we will provide guidance and conversions.

## Design options considered

### Option A — Feature-gate everything, two parallel internal models

Literally follow the #357 sketch: add an `http-types` feature, and `#[cfg]`-gate
every type alias, import, and impl so the same public item names are backed by
either `http-types` or `http`/our-own types.

- **Pros:** Maximum source-compatibility for existing users when the feature is
  off *and* on; one published item set.
- **Cons:** As #357 itself predicts, "it is quite likely to make the code very
  annoying to read." Every header/body/version touch point becomes a
  `#[cfg]`-forked pair. Doc-tests, signatures (`impl Into<Body>` vs our `Body`),
  and serialization (`header_serde`) all fork. High maintenance cost, easy to let
  one path rot.

### Option B — Native `http` by default; `http-types` feature re-exports the old surface; introduce a crux-owned `Body`

Default path is rewritten to use `http` types plus a small crux-owned `Body`
type (wrapping bytes) and `mime` for content types. The `http-types` feature
swaps the internal representation back to `http-types` and restores the old
re-exports/conversions for users who need them.

- **Pros:** The default path is clean and idiomatic; new users get `http` with
  no ceremony. The messy `#[cfg]` forking is concentrated at a small number of
  boundary types (`Request`, `Response`/`ResponseAsync`, header (de)serialization,
  `Body`, `content_type`) rather than smeared across the whole crate.
- **Cons:** Still two code paths to maintain for the lifetime of the feature.
  The crux-owned `Body` must re-implement the ergonomic `Into<Body>` conversions.

### Option C — Bite the bullet, `http` only, conversion traits, no `http-types` feature

Drop `http-types` entirely in one step. Provide `From`/`TryFrom` conversion
helpers to ease migration, but make everyone move.

- **Pros:** Simplest end state; no dual maintenance; fastest to a clean crate.
- **Cons:** Hard breaking change with no opt-out, contradicting the explicit
  request in #357/#285 to offer a backwards-compatible feature flag. Riskier for
  existing apps.

### Recommendation

**Option B.** It delivers the stated objective — `http` by default,
`http-types` available as a compatibility feature — while keeping the dual-path
complexity contained to a handful of well-understood boundary modules instead of
the whole crate. Option C remains the fallback if Option B's dual maintenance
proves "far too messy" (the escape hatch #357 anticipates): we can later
deprecate and remove the `http-types` feature.

## Proposed design (Option B)

### Cargo features

```toml
[features]
default = ["encoding"]
encoding = ["encoding_rs", "web-sys"]
typegen = ["crux_core/typegen"]
facet_typegen = ["crux_core/facet_typegen"]
# Opt-in: restore the legacy http-types-based representation and public surface.
http-types = ["dep:http-types", "dep:mime"]   # name TBD; see open questions

[dependencies]
http = "1.4"                       # now a default (non-optional) dependency
mime = "0.3"                       # replaces http_types::Mime on the default path
bytes = "1"                        # optional: backing store for the Body type
http-types = { package = "http-types-red-badger-temporary-fork", version = "6.0", default-features = false, optional = true }
# `http-compat` feature is removed.
```

Notes:

- `http` becomes a normal dependency (it is tiny — only `bytes` + `itoa`).
- The `http-compat` feature and the optional `http` dependency it gated are
  removed; the conversions it provided are no longer needed because the default
  types are already `http` types.
- The `http-types` dependency becomes `optional = true` and is only compiled when
  the `http-types` feature is enabled.

### Public surface (`lib.rs`)

```rust,ignore
pub use http;           // always the real `http` crate
pub use http::Method;
pub use url::Url;       // unchanged
pub use mime;           // crux_http::mime (content types)
```

Under the `http-types` feature, `http_types` is additionally re-exported for
users who need to bridge between `crux_http` types and legacy `http-types` code:

```rust,ignore
#[cfg(feature = "http-types")]
pub use http_types;
```

There is no conditional re-export of `crux_http::http` — it always means the
real `http` crate.

### A crux-owned `Body` type

Introduce `crux_http::Body` to replace `http_types::Body`. Because the protocol
collapses bodies to `Vec<u8>` anyway, this can be a simple, synchronous,
in-memory type:

```rust,ignore
pub struct Body {
    bytes: bytes::Bytes,
    mime: Option<mime::Mime>,
}
```

`bytes::Bytes` is chosen over `Vec<u8>` for its `O(1)` clone — `Request` is
cloned on every middleware hop — and because `http` already pulls `bytes` into
the dependency graph, so there is no new transitive dependency. It should provide
`Into<Body>` conversions for `String`, `&str`, `Vec<u8>`, `&[u8]`, and
`serde_json::Value`, plus `into_bytes() -> bytes::Bytes`, `mime()`, `len()`,
and `is_empty()`. The async `AsyncRead`/streaming aspects of `http_types::Body`
are not used by the effect contract and are not carried over.

### `Request` and `ResponseAsync`

These wrapper structs currently hold an `http_types::Request` /
`http_types::Response`. They are rewritten with fields held directly:
`method: http::Method`, `url: url::Url`, `headers: http::HeaderMap`, `body:
Body` (and `status: http::StatusCode`, `version: Option<http::Version>` for
responses). There is a single implementation — no `#[cfg]` forking.

The public methods (`header`, `set_header`, `body`, `take_body`, `url`,
`method`, `query`/`set_query`, iterators, `content_type`, …) are implemented
over `http::HeaderMap` + `url::Url` + `Body`. `query`/`set_query` use `serde_qs`
(already a dependency).

The `AsRef`/`AsMut`/`From`/`Into` impls now target `http::Request<Body>` and
`http::HeaderMap` unconditionally — the native version of what `http-compat` did,
but lossless and bidirectional. Under the `http-types` feature, additional
conversion impls for `http_types::Request` and `http_types::Response` are added
(gated with `#[cfg(feature = "http-types")]`) so users can bridge between the two
ecosystems. This is the only place `#[cfg(feature = "http-types")]` appears.

### `Response<Body>` and header (de)serialization

`Response<Body>` stores `version: Option<http::Version>`,
`status: http::StatusCode`, and headers as `http::HeaderMap`. The custom
`header_serde` module (which serializes headers as `Vec<(String, Vec<String>)>`)
is re-pointed at `HeaderMap`. The `new_headers()` hack in `response/mod.rs` (which
constructs an `http_types::Headers` by spinning up a throwaway `Request`) is
replaced by `http::HeaderMap::new()` — a strict simplification.

Note `http::StatusCode` and `http::Version` do not implement `facet::Facet`.
`http::Version` is used only on the `Response`/`ResponseAsync` façade, which is
never type-generated, so that is fine. The one `Facet`-deriving type that
referenced a status code, `HttpError::Http { code, .. }`, is already
`#[facet(skip)]` and `#[serde(skip)]` (internal only); we additionally change
`code` to a plain `u16` and drop `#[facet(opaque)]` (see `error.rs` below and
"Resolved decisions").

### `error.rs`

- Remove `From<http_types::Error>`.
- Change `HttpError::Http.code` from `http_types::StatusCode` to a plain `u16`,
  dropping `#[facet(opaque)]` but keeping the variant's `#[facet(skip)]` +
  `#[serde(skip)]` (see "Resolved decisions").
- `decode.rs` stops importing `http_types::Error`; its `DecodeError` is already
  self-contained.

### `middleware/redirect.rs`

Re-point `StatusCode`, header `LOCATION` access, and the request URL mutation
onto `http`/`url` types. The `http_types::url::ParseError` arm becomes
`url::ParseError` (the `url` crate is a direct dependency).

### `protocol.rs`

`From<HttpResponse> for ResponseAsync` currently builds an `http_types::Response`.
On the default path it builds the native `ResponseAsync` from `status`, `headers`,
`body` directly. `into_protocol_request` is largely unchanged (it already reads
method/url/headers/body generically), but the body extraction uses the new `Body`
API instead of `http_types::Body::into_bytes`.

### What stays identical

- `HttpRequest`, `HttpResponse`, `HttpResult`, `HttpHeader` and their typegen /
  serde representations.
- The `command::Http` builder method set (`get`/`post`/…/`request`).
- The middleware trait shapes.
- `crux_http::Url` (always `url::Url`) and `crux_http::Method` (name preserved;
  underlying type changes from `http_types::Method` to `http::Method`).

## Migration impact for users

| Scenario | Action |
| --- | --- |
| App only uses `crux_http::{get, post, RequestBuilder, Response, body_json, ...}` and `crux_http::Method` | Likely compiles unchanged; `Method` is now `http::Method` (API-compatible for common uses). |
| App imports `crux_http::http::mime::*` | Update to `crux_http::mime::*` or `mime::*` directly. |
| App imports other `crux_http::http::*` items (e.g. `Body`, `Headers`, `Version`) | Update to `http::*` or `crux_http::Body` as appropriate. |
| App used the `http-compat` feature | Feature removed; the default types are now `http` types, so `TryFrom`/`TryInto` round-trips are no longer needed — use the types directly. |
| App has code that constructs or consumes `http_types::Request`/`Response` and needs to pass them to/from `crux_http` | Enable the `http-types` feature; conversion impls are provided. |
| App relied on streaming `http_types::Body` / `AsyncRead` | The streaming body model is not carried over; refactor to use the in-memory `crux_http::Body`. |

We should ship:

- A `CHANGELOG.md` entry calling out the `crux_http::http` re-export change and
  the removal of `http-compat`.
- A short migration section (in the book / crate docs) with before/after
  snippets, including how to turn on `http-types`.

## Implementation plan

All design questions are resolved (see "Resolved decisions"). Work through the
tasks below in order — the groupings reflect natural dependencies within the
change, not separate releases.

### 1. Pre-flight cleanup

Small, isolated changes. Good to do first to keep the diff clean.

- `error.rs` — `HttpError::Http.code: u16`, drop `#[facet(opaque)]`, remove
  `From<http_types::Error>`, update `test_error_display`.
- `expect.rs` — `http_types::convert::DeserializeOwned` →
  `serde::de::DeserializeOwned`.
- `response/decode.rs` — drop the unused `http_types::Error` import.

### 2. Introduce `crux_http::Body`; remove `http_types::Body`/`Mime` from public API

Still entirely backed by `http-types` internally — no feature-flag forking yet.
This step removes `http_types::Body` and `http_types::Mime` from all public
signatures, which shrinks what needs `#[cfg]`-gating in the next step.

- Add `src/body.rs` — `pub struct Body { bytes: Vec<u8>, mime:
  Option<mime::Mime> }` with `Into<Body>` impls for `String`, `&str`, `Vec<u8>`,
  `&[u8]`, `serde_json::Value`; plus `into_bytes()`, `mime()`, `len()`,
  `is_empty()`.
- Add `mime = "0.3"` to `[dependencies]`.
- `crux_http::Request` — store body as `crux_http::Body` alongside the wrapped
  `http_types::Request` (not inside it); `set_body`/`take_body` operate on the
  wrapper field. The `http_types::Request` is retained only for method, URL,
  headers, and query helpers.
- `request_builder.rs` / `command.rs` — `body(impl Into<Body>)` and
  `content_type(impl Into<mime::Mime>)` use our types; `http_types::Body` and
  `http_types::Mime` no longer appear in any public signature.
- `protocol.rs` — `into_protocol_request` takes body bytes from the wrapper
  directly (synchronous); the `async http_types::Body::into_bytes()` call is
  removed.
- `lib.rs` — `pub use crate::body::Body`; `pub use mime`.

### 3. Add `http-types` feature; implement native `http` path; remove `http-compat`

After step 2, the remaining `http-types` surface is the "frame" types: headers,
method, status, version, and the `http_types::Request`/`Response` wrappers.

- `Cargo.toml` — `http = "1.4"` becomes a default dependency; `http-types` becomes
  `optional = true` behind the new feature; remove `http-compat` and its gated
  `dep:http`.
- `lib.rs` — conditional re-exports (`pub use http` vs `pub use http_types as http`,
  `http::Method` vs `http_types::Method`).
- `response/mod.rs` — `http::HeaderMap::new()` replaces the throwaway-`Request`
  hack.
- `response/response.rs` — `http::StatusCode`, `http::Version`,
  `http::HeaderMap`; update `header_serde`; replace `http-compat` `TryInto` with
  a native lossless conversion.
- `response/response_async.rs` — hold `status: http::StatusCode`,
  `headers: http::HeaderMap`, `body: Body` directly on the default path instead
  of wrapping `http_types::Response`.
- `request.rs` — replace the wrapped `http_types::Request` with
  `method: http::Method`, `url: url::Url`, `headers: http::HeaderMap`;
  re-implement header methods and `query`/`set_query` over `serde_qs`; update
  `AsRef`/`AsMut`/`From`/`Into` impls to target `http::Request<Body>` and
  `http::HeaderMap`.
- `config.rs` — header types.
- `client.rs` — `Method`/`Url` types.
- `middleware/redirect.rs` — `http::StatusCode`, `http::header::LOCATION`,
  `url::ParseError`.
- `protocol.rs` — `From<HttpResponse> for ResponseAsync` constructs
  `ResponseAsync` fields directly.
- `src/compat.rs` (new, `#[cfg(feature = "http-types")]` only) — conversion
  impls between `crux_http::Request`/`Response`/`ResponseAsync` and
  `http_types::Request`/`Response`.

### 4. Tests and CI

- Confirm all existing tests pass on the default (`http`) path. The `protocol.rs`
  insta snapshots must be byte-for-byte unchanged — the primary guard that the
  migration is bridge-invisible.
- Add a CI job: `cargo test --features http-types` for `crux_http`.
- Add round-trip integration tests:
  - `http::Request<Body>` → `crux_http::Request` → `HttpRequest`
  - `HttpResponse` → `crux_http::Response<Vec<u8>>` → `http::Response<Vec<u8>>`
- Build the `counter-http`, `counter-middleware`, `weather`, and `notes` examples
  on both feature configurations.

### 5. Docs and release

- `CHANGELOG.md` — `crux_http::http` now re-exports the `http` crate (not
  `http-types`); `http-compat` feature removed; how to enable `http-types` for
  backwards compatibility.
- Book migration section — before/after snippets for the common cases in the
  "Migration impact" table above.

## Testing strategy

- Existing unit/integration tests (`tests/command_with_shell.rs`,
  `tests/command_with_tester.rs`, `client.rs` tests, `protocol.rs` snapshot
  tests) must pass on the **default** (`http`) path. The `protocol.rs` insta
  snapshots assert the wire format and should be unchanged — a strong guard that
  the migration is bridge-invisible.
- Add a CI job that builds and tests with `--features http-types` to keep the
  legacy path alive.
- Add round-trip tests: `http::Request<Body> -> crux_http::Request ->
  HttpRequest` and `HttpResponse -> crux_http::Response -> http::Response<_>` to
  lock in the native conversions that replace `http-compat`.

## Resolved decisions

- **Feature name is `http-types`**, matching the crate name.

- **`Body` backing store is `bytes::Bytes`.** `http` already brings `bytes` into
  the dependency graph, so there is no new transitive dependency. The key benefit
  over `Vec<u8>` is `O(1)` clone, which matters because `Request` is cloned on
  every middleware hop.

- **`crux_http::http` always re-exports the real `http` crate.** Features must be
  additive; the `http-types` feature therefore only adds the `crux_http::http_types`
  re-export and conversion impls. It does not change any default types or
  re-exports. The internal representation is `http`-based on all paths — there is
  no `#[cfg]` forking of the core types.

- **`http` is emscripten-compatible.** The 128-bit issue that forced the
  `http-types` fork (#195) was `http-types`' `trace_id` field. The `http` crate
  has only two runtime dependencies (`bytes`, `itoa`) and contains no 128-bit
  integers. The fork can be dropped entirely from the default build.

- **`HttpError::Http.code` is stored as `u16`.** `http::StatusCode` is not a
  `Facet` type, so a `u16` lets us drop the `#[facet(opaque)]` workaround and
  matches the `status: u16` already on `HttpResponse`. We still keep the
  variant's `#[facet(skip)]` + `#[serde(skip)]` — that's there because the `Http`
  variant is internal-only (never reported by the shell), independent of the
  field type. Callers wanting a typed status can use
  `http::StatusCode::from_u16(code)`.

## Open questions

1. **Deprecation timeline.** Do we commit up front to removing the `http-types`
   feature in a future release (Option C end state), or keep it indefinitely?

## Appendix: file-by-file change checklist

- `Cargo.toml` — feature table + dependency changes (above).
- `lib.rs` — re-export switch (`http` vs `http_types`), `Method` source, `mime`
  re-export.
- `request.rs` — internal representation, `Body`/header methods, `AsRef`/`AsMut`/
  `From`/`Into` impls, `query`/`set_query`.
- `request_builder.rs` — `Body`/`Mime`/`DeserializeOwned` imports; `content_type`,
  `body`, `body_json`, `body_form`, `body_string`, `body_bytes`.
- `command.rs` — same imports; builder body/content-type methods.
- `config.rs` — `Url` + headers imports.
- `client.rs` — `Method`/`Url` imports.
- `response/mod.rs` — replace `new_headers()` hack with `HeaderMap::new()`.
- `response/response.rs` — `Version`/`StatusCode`/`HeaderMap`, `header_serde`,
  drop/native-ize `http-compat` `TryInto`.
- `response/response_async.rs` — internal representation, body methods,
  `From`/`Into` impls.
- `response/decode.rs` — drop `http_types::Error`.
- `error.rs` — `HttpError::Http.code` type, drop `From<http_types::Error>`.
- `expect.rs` — `convert::DeserializeOwned` → `serde::de::DeserializeOwned`.
- `middleware/redirect.rs` — `StatusCode`/`LOCATION`/url-mutation/parse-error.
- `protocol.rs` — `From<HttpResponse> for ResponseAsync`, body extraction.
- CI — add `--features http-types` job.
- `CHANGELOG.md` + book — migration guide.
