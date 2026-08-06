# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.20.0](https://github.com/redbadger/crux/compare/crux_http-v0.19.0...crux_http-v0.20.0) - 2026-08-06

### Added

- **`HttpError::body`, `HttpError::body_json` and `HttpError::code`** — read what the
  server said when it rejected a request, without destructuring the variant.

  A 4xx/5xx arrives as `Err(HttpError::Http { code, message, body })`, and `body` has
  always held the server's own explanation. But `Display` shows only the status
  (`"HTTP error 409: 409 Conflict"`), so getting at the message meant hand-rolling
  this in every app:

  ```rust
  // before
  let crux_http::HttpError::Http { body: Some(body), .. } = error else { return None };
  serde_json::from_slice::<serde_json::Value>(body)
      .ok()
      .and_then(|b| b.get("error").and_then(|e| e.as_str()).map(String::from))

  // after
  error.body_json::<serde_json::Value>().ok()
      .and_then(|b| b["error"].as_str().map(String::from))
  ```

  `body_json` deserializes into whatever shape your API uses — your own envelope, an
  RFC 7807 `problem+json` struct, or `serde_json::Value`. Note that body decoding is
  skipped for an error status, so the raw error body survives even for a request
  built with `expect_json::<T>()`; there is now a test pinning that.

- **`HttpError::header`, `HttpError::headers` and `HttpError::content_type`** — read the
  headers of the response that was rejected.

  A rejection's *policy* often lives only in its headers, and none of it is recoverable
  from the status or the body: `Retry-After` on a 429 or 503, `WWW-Authenticate` on a 401
  (expired token vs insufficient scope), rate-limit headers, or the `Content-Type` that
  says whether an error body is JSON, an RFC 7807 document, or a proxy's HTML page.
  `Response::new` used to drop the `HeaderMap` on the error branch, and since `crux_http`
  middleware does not run in the command API, an app had no way to see any of it.

  ```rust
  if let Some(retry_after) = error.header("retry-after") { /* back off politely */ }
  ```

  This adds a `headers` field to `HttpError::Http` — see the breaking change below.

- **`crux_http::testing::rejection(status, body)`** — builds the
  `crux_http::Result<Response<Body>>` a feature receives when the server rejects a
  request, via the same conversion a real shell response takes:

  ```rust
  let event = Event::Saved(crux_http::testing::rejection(409, r#"{"error":"…"}"#));
  ```

  Use it wherever you would previously have fabricated `Ok(response_with_409)` (see
  the breaking change below).

- **`crux_http::testing::rejection_from(HttpResponse)`** — the header-carrying form,
  taking the same protocol response you would resolve a request with in an end-to-end
  test, so both styles of test describe a rejection the same way:

  ```rust
  let result = rejection_from(HttpResponse::status(429).header("retry-after", "30").build());
  ```

  `rejection(status, body)` is unchanged, and is now sugar over it.

- `crux_http::testing` now has module docs, stating the invariant the builders divide:
  `ResponseBuilder` for the `Ok(Response)` of a successful exchange, `rejection` /
  `rejection_from` for the `Err` of a rejection. There is no third case.

- `HttpRequestBuilder::body_json`, which sets a JSON body **and**
  `content-type: application/json`, mirroring what
  `command::RequestBuilder::body_json` puts on the wire.

  The protocol builders are how a test names the request it expects, but
  `HttpRequestBuilder::json` sets only the body, where the capability side sets the
  mime too (via `Body::from_json`). Mirroring a real request therefore failed on
  the `content-type` header alone unless you knew to add it by hand:

  ```rust
  // before — passes only with the header spelled out
  assert_eq!(
      &request.operation,
      &HttpRequest::post(URL)
          .header("content-type", "application/json")
          .json(&body)
          .build()
  );

  // after
  assert_eq!(&request.operation, &HttpRequest::post(URL).body_json(&body).build());
  ```

  `json` is unchanged and still sets only the body: these builders construct
  protocol-layer values, so they must stay able to express a JSON body with no
  `content-type` (or a malformed one), and changing `json` would silently break
  tests that already add the header themselves.

### 💥 Breaking Changes

- **`HttpError` has two new variants, and `HttpError::Http` now means only a server
  rejection.** Two things that were not rejections used to report themselves as one:

  | Was | Is now |
  | --- | --- |
  | `Response::body_bytes` on an already-taken body → `Http { code: <the success status>, headers: <that response's>, .. }` | `HttpError::BodyAlreadyTaken` |
  | A shell status outside 100–999 → `Http { code: 999, .. }` | `HttpError::InvalidStatusCode(999)` |

  So `error.code()` could return `Some(200)`, and `matches!(err, HttpError::Http { .. })`
  was not a reliable test for "the server rejected this". Both now are:

  ```rust
  // `Some` if and only if the server rejected the request
  if let Some(code) = error.code() { … }
  ```

  `BodyAlreadyTaken` carries nothing: the caller still holds the `Response`, so the status
  and headers the old error reported were already in hand — and they described a *successful*
  response, which is what made them misleading.

  An exhaustive `match` on `HttpError` needs two new arms. That is deliberate: the enum is
  **not** `#[non_exhaustive]`, so a new failure mode is a compile error you get to think
  about, rather than something that silently lands in a wildcard.

- **`From<http_types::Error> for HttpError` is gone** (`http-types` feature only). It mapped
  a middleware error onto `HttpError::Http`, which now means only a server rejection, and it
  has no callers inside the crate. `http-types` middleware must map its own errors
  explicitly.

  It isn't rehomed onto a new variant because **the `http-types` feature is slated for
  removal in full** — `http` is the only HTTP type system `crux_http` will keep. This is the
  first piece to go; `crux_http::compat` and the `pub use http_types` re-export follow in
  their own release. If you still depend on the feature, now is the time to say so.

- **`HttpError::Http` has a new `headers` field**, so that a rejection's headers reach the
  app (see `HttpError::header` above). It is `Box<HeaderMap>`, boxed only because a bare
  `HeaderMap` is 96 bytes and this type is the `Err` of nearly every function in the crate —
  inline, it pushed `HttpError` to 160 bytes and tripped `clippy::result_large_err` across
  the crate.

  Neither `headers` nor `body` is optional, and `body` is now a plain `Vec<u8>` rather than
  `Option<Vec<u8>>`. `Response::new` is the variant's only constructor and always supplies
  both, so the `Option`s described states that could not arise. Both types are
  niche-optimised, so this costs nothing at runtime — `HttpError` stays 72 bytes — and the
  accessors are unchanged: `body()` still returns `None` for an empty body, and `headers()`
  still returns `None`, but now that means simply "not a rejection".

  `match` arms that name only the fields they use are unaffected:

  ```rust
  Err(HttpError::Http { code, .. }) if code == 401 => { … }      // still compiles
  ```

  The variant is also now **`#[non_exhaustive]`**, so this is the last time a new field
  breaks you: only `crux_http` constructs it, and what a rejection carries can grow behind
  the accessors. Code that *constructed* it — in practice, tests — must switch to
  `crux_http::testing::rejection` / `rejection_from`, which build it through the real
  conversion:

  ```rust
  // before
  let error = HttpError::Http { code: 409, message: "409 Conflict".into(), body: Some(body) };
  // after
  let error = crux_http::testing::rejection::<Vec<u8>>(409, body).unwrap_err();
  ```

  Note that `HttpError`'s `PartialEq` now compares headers too, so two rejections that
  differ only in what the server sent in its headers are no longer equal. Compare against
  `rejection` / `rejection_from` rather than a value assembled by hand.

- **`ResponseBuilder::with_status` now panics for a 4xx or 5xx status.** It could
  previously build a `Response` carrying an error status — a value no app can ever
  receive, because `crux_http` converts those responses into
  `Err(HttpError::Http { .. })` before the event is sent.

  This mattered in practice. A downstream app had eight call sites shaped like this:

  ```rust
  match result {
      Ok(mut response) => {
          if response.status().is_success() { /* … */ }
          else { show(api::error_reason(&mut response)) }  // unreachable
      }
      Err(error) => fail(&error),  // "HTTP error 409: 409 Conflict"
  }
  ```

  The `else` branch cannot run, so users saw the bare status instead of the sentence
  the service had written for them. Seven of those sites had **passing tests** for the
  message, because each test built the impossible value —
  `Ok(ResponseBuilder::with_status(409).body(…).build())` — and asserted the dead
  branch. Code review passed all eight.

  The panic turns exactly those tests red, with a message naming the replacement.
  Migration is mechanical:

  ```rust
  // before — asserts a state the app can never observe
  let result = Ok(ResponseBuilder::with_status(409).body(body).build());
  // after — what the app really receives
  let result = crux_http::testing::rejection(409, body);
  ```

  Non-error statuses (1xx, 2xx, 3xx) are unaffected, including non-standard ones.
  Nothing about the runtime behaviour of a request changes — this is a test-helper
  guard rail plus documentation of an invariant `crux_http` already had.

### Documentation

- `Response`, `Response::status`, `HttpError::Http` and the crate root now state the
  invariant plainly: an `Ok(Response)` never carries a 4xx or 5xx status, so a
  rejection can only be handled on the `Err` side. The `Response` docs carry the
  correct match shape as a compiled example.

- `command::RequestBuilder::middleware` now warns that it is a **no-op**: nothing on the
  command API executes a middleware stack, so middleware pushed there — `Redirect`
  included — is accepted and silently ignored. Its example previously implied redirects
  were followed. This documents existing behaviour; the underlying gap is
  [#556](https://github.com/redbadger/crux/issues/556).

### ⚙️ Miscellaneous Tasks

- Align with `crux_core` 0.20.0. Upgrade the other capability crates
  (`crux_kv` 0.14.0, `crux_time` 0.18.0) together with this one — a capability left on
  `crux_core` 0.19 would pull a second, incompatible `crux_core` into the same tree.
- Dependency updates, including `http` 1.4 -> 1.5.

## [0.19.0](https://github.com/redbadger/crux/compare/crux_http-v0.18.0...crux_http-v0.19.0) - 2026-07-06

> **📖 See the [Migrating `crux_http` to native `http` types](https://redbadger.github.io/crux/guide/migrate-crux-http.html)
> guide in the book for a step-by-step walkthrough of the changes below.**

### 💥 Breaking Changes

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

---

**`insert_header` and `append_header` now take `http::HeaderValue` directly.**

The low-level header mutation methods on `Request`, `RawResponse`, and `Response<T>` now
accept `HeaderValue` instead of `impl AsRef<str>`. The return types also change to mirror
`http::HeaderMap`: `insert_header` returns `Option<HeaderValue>` (the evicted previous
value, if any) and `append_header` returns `bool` (whether a prior value already existed
for that name).

This eliminates silent failure: the previous `impl AsRef<str>` signature silently discarded
any value that failed `HeaderValue::from_str`, with no indication to the caller.

The high-level builder `.header(name, value)` on `RequestBuilder` and `ResponseBuilder`
is **unchanged** — it still accepts `impl AsRef<str>` for convenience and panics on
invalid values.

```rust
// Before
req.insert_header("authorization", format!("Bearer {token}"));
res.append_header("set-cookie", "sid=abc");

// After
use crux_http::http::HeaderValue;
req.insert_header(
    "authorization",
    HeaderValue::from_str(&format!("Bearer {token}")).expect("valid token"),
);
res.append_header("set-cookie", HeaderValue::from_static("sid=abc"));
```

---

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

---

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

---

**`Config::add_header` name parameter is now `impl IntoHeaderName`.**

`Config::add_header` previously accepted `impl AsRef<str>` for the header name and
returned `Err` if the name string was invalid. The parameter is now
`impl http::header::IntoHeaderName`, consistent with every other header-setting method
in the library. The `Result` return is kept — it now only reflects validation of the
value.

Callers using `&'static str` literals are unaffected. Callers who were passing a
runtime-built `String` must convert it to `HeaderName` first:

```rust
use http::HeaderName;
// Before
config.add_header(format!("x-{key}"), "value")?;
// After
let name = HeaderName::from_bytes(format!("x-{key}").as_bytes())?;
config.add_header(name, "value")?;
```

---

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

**`Request::set_header` is deprecated — use `insert_header` instead.**

`set_header` was an `http-types`-era alias that did nothing but delegate to
`insert_header`. It is now `#[deprecated]`. Replace all call sites:

```rust
// Before
req.set_header("x-trace-id", HeaderValue::from_static("abc"));
// After
req.insert_header("x-trace-id", HeaderValue::from_static("abc"));
```

### 🐛 Bug Fixes

- **Multi-value headers are no longer silently clobbered in `http-types` compat
  conversions.** The `From<http_types::Request> for Request` and
  `From<http_types::Response> for RawResponse` conversions were calling `insert`
  inside the inner loop that iterates over per-name values, so when a header had more
  than one value only the last one survived. They now call `append`, preserving all
  values.

### 🚀 Features

- **`header_all(name)` on `Request`, `RawResponse`, and `Response<T>`** — all three
  types now expose `header_all`, returning all values for a given header name as an
  `http::header::GetAll` iterator. Useful when a header can legitimately appear multiple
  times (e.g. `Set-Cookie` in responses, `Accept` or `Cookie` in requests). `Request`
  previously had no equivalent to the `header_all` already present on the response types.
- **New `crux_http::Body` type** — a simple, synchronous, in-memory request body with
  an optional MIME type. Replaces the async `http_types::Body`. Provides `Into<Body>`
  conversions from `String`, `&str`, `Vec<u8>`, `&[u8]`, and `serde_json::Value`, plus
  `into_bytes()`, `mime()`, `len()`, and `is_empty()`.
- **Native `http` conversions** — `From<http::Request<Body>> for crux_http::Request`,
  `From<crux_http::Request> for http::Request<Body>`, and
  `TryFrom<crux_http::Response<Body>> for http::Response<Body>` are available out of the
  box without any feature flag.
- **`http-types` compat feature** — add `crux_http = { features = ["http-types"] }` to
  your `Cargo.toml` to get `From`/`Into` impls between `crux_http` types and the legacy
  `http_types` types, as a bridge while migrating.
- **`ResponseBuilder::append_header(name, value)`** — the test helper now provides
  `append_header` alongside the existing `header` method, so tests can build responses
  with multiple values for the same header name (e.g. multiple `Set-Cookie` headers)
  without losing earlier values.
- **`crux_http::mime` re-export** — the `mime` crate (v0.3) is now re-exported directly
  as `crux_http::mime`, giving access to constants like `mime::APPLICATION_JSON` and
  `mime::TEXT_HTML` without needing a separate dependency.

### ⚙️ Miscellaneous Tasks

- `http_types` (the temporary fork) is no longer a default dependency; it is pulled in
  only when the `http-types` feature is enabled, reducing the default dependency footprint.
- `into_protocol_request` is now a synchronous function (was `async`); the previous
  `await` on `http_types::Body::into_bytes()` is gone.
- `ResponseAsync` renamed to `RawResponse` throughout.

### 📝 Notes

**HTTP error status codes (4xx, 5xx) arrive as `HttpResult::Ok`, not `HttpResult::Err`.**

This is intentional and unchanged from previous versions. `HttpResult::Err` signals only
a *transport-level* failure — the shell could not complete the exchange at all (bad URL,
IO error, or timeout). Any completed HTTP exchange, regardless of status code, is
`HttpResult::Ok(HttpResponse { status, … })`.

To handle error status codes, inspect `response.status`:

```rust
match result {
    HttpResult::Ok(response) if response.status == 200 => { /* success */ }
    HttpResult::Ok(response) if response.status >= 400 => { /* HTTP error */ }
    HttpResult::Ok(_) => { /* other */ }
    HttpResult::Err(e) => { /* transport failure: bad URL, IO error, or timeout */ }
}
```

The three `HttpError` variants that cross the FFI boundary are `Url`, `Io`, and `Timeout`.

Note: this is the protocol-level view. In Rust app code, `Response::new()` converts
4xx/5xx responses into `Err(HttpError::Http { code, .. })`, so that layer does surface
them as errors — just not via the FFI.

---

**`protocol::HttpRequest` header fields are unvalidated strings — shell authors must not assume validation has occurred.**

`HttpRequest` is a cross-language data carrier. Its header `name` and `value` fields are
raw strings with no HTTP-spec validation. The high-level [`RequestBuilder::header`] in
app code *does* validate values via the `http` crate and panics on invalid input, but
that validation happens before the request is serialised into `HttpRequest`; it is not
a property of the protocol type itself.

`HttpRequestBuilder::header` (used when constructing test fixtures) accepts any string
by design — including deliberately malformed values. Shell authors: pass headers straight
to your underlying HTTP client and let it apply its own rules.

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
