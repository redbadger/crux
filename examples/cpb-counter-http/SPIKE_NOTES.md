# Spike: counter_http on the Circuit Playground Bluefruit, over BLE

Throwaway spike, following `examples/cpb-counter`. Evidence for an RFC, not mergeable code.

The goal is `examples/counter-http` (a shared counter on crux-counter.fly.dev: GET, POST
/inc and /dec with an optimistic update, SSE) running as no_std firmware, using
**crux_http's own API**. The board has no IP stack, so its HTTP and SSE effects go over
BLE to a Chrome page. That page is itself a Crux app, which acts as a gateway and performs
them with fetch.

## Phase 1: a no_std subset of crux_http

### Result

`cargo check -p crux_http --no-default-features --target thumbv7em-none-eabihf` passes.
With default features nothing changes: `cargo test -p crux_http` gives the same counts as
before (97 unit, 15 integration, 76 doc), `cargo test --workspace` passes (483), the
`shared` tests of counter-http, counter-middleware, counter-routing and weather pass, and
counter-http's typegen produces byte-identical TypeScript.

The no_std path has its own test, `crux_http/tests/nostd_command.rs`
(`cargo test -p crux_http --no-default-features --test nostd_command`, 7 tests). It
compiles to nothing under std, so it can only run against the no_std code.

### Blockers, measured

| Dependency | no_std? | What happened |
|---|---|---|
| `http` 1.5 | **no** | `compile_error!("std feature currently required, support for no_std may be added later")` in its `lib.rs` |
| `mime` 0.3 | **no** | No features at all, uses std |
| `serde_qs` 1.1 | no | Only used by `.query()` |
| `async-trait` | n/a | Only used by `middleware` and the std `EffectSender` |
| `encoding_rs` | yes | But only used by `body_string`'s charset decoding, which is std-only anyway |
| `url` 2.5.8 | **yes** | `default-features = false`, `Url::parse`/`join` work |
| `serde_json` | yes | `features = ["alloc"]` |
| `serde_bytes` | yes | `features = ["alloc"]` |
| `derive_builder` 0.20 | **yes, on stable** | `default-features = false, features = ["alloc"]` plus `#[builder(no_std)]`. Its docs say nightly-only, but that is out of date |
| `thiserror` 2 | yes | |
| `facet-generate-attrs` 0.18 | **no (our own crate)** | It depends on `facet` with default features, which turns `facet/std` on for the whole build: 2,787 errors in facet-core on thumbv7em. **Upstream ask:** `default-features = false` there |

`http` and `mime` are not peripheral. They are in crux_http's public API: `Request` is built
on `HeaderMap`/`Method`, `Response` keeps `StatusCode`/`HeaderMap`/`Version`, `HttpError::Http`
carries a `Box<HeaderMap>`, and the crate does `pub use http` and `pub use mime`.

### What was done (the "subset" option)

The options were patching no_std forks of `http`/`mime` in (same API, a fork to maintain),
replacing them with crux_http-owned types (breaking for every user), or a no_std subset.
This spike does the subset:

- New `std` feature in `default`, carrying `http`, `mime`, `serde_qs`, `async-trait`,
  `facet-generate-attrs` and the std features of the rest. `encoding`, `facet_typegen` and
  `http-types` imply it. crux_core is depended on with `default-features = false`, and its `std`
  comes back through ours.
- `facet` and `thiserror` spell out versions instead of `.workspace = true`, the same
  workspace-dependency trap as cpb-counter's SPIKE_NOTES §5.
- **Shared between std and no_std, the same code:** `protocol` (`HttpRequest`,
  `HttpResponse`, `HttpResult`, `HttpHeader` and their builders), `expect`, and `HttpError`.
  - `HttpRequestBuilder::body_json` now writes `content-type: application/json` as literals
    rather than via `http::header::CONTENT_TYPE` and `mime::APPLICATION_JSON`, which are the same bytes.
  - `#[facet(typegen::bytes)]` became `#[cfg_attr(feature = "std", facet(typegen::bytes))]`.
  - `HttpError` stays one type. Only the skipped `Http` variant's `headers` field differs:
    `Box<HeaderMap>` under std, `Vec<HttpHeader>` without. So does the accessor:
    `header(&str) -> Option<&str>` and `headers() -> Option<&[HttpHeader]>` without std.
    The **serialized** variants (`Url`, `Io`, `Timeout`) are identical, so a no_std device
    and a std gateway agree on the wire.
  - `HttpResult` therefore serializes the same in both.
- **std only:** `client`, `middleware`, `config`, the legacy `Request`/`RequestBuilder`,
  `Body`, `RawResponse`, `testing`, `shell`, `compat`, `.query()`, `.content_type(Mime)`,
  `body_form`, and the `http`/`mime`/`Method` re-exports.
- **no_std twins, at the same paths** (`src/nostd/`, swapped in with `#[path]`):
  - `crux_http::Response<T>`: `status: u16`, `headers: Vec<HttpHeader>`, `body: Option<T>`,
    with `status()`, `header(&str)`, `headers()`, `content_type() -> Option<&str>`, `body()`,
    `take_body()`, `with_body()`, `body_bytes()`, `body_string()` (UTF-8 only), `body_json()`,
    `PartialEq`/`Eq`, and `TryFrom<HttpResponse>`. The same rules apply as in std: 4xx/5xx
    becomes `HttpError::Http` (with a `message` of just the number, because there is no reason-phrase
    table), a status outside 100..=999 becomes `InvalidStatusCode`, and a non-UTF-8 body
    becomes `HttpError::Io("could not decode body as utf-8")`, the variant and message std produces.
  - `crux_http::command::{Http, RequestBuilder}`: `Http::get/head/post/put/delete/patch/
    options/trace/connect(impl AsRef<str>)`, and `Http::request(method: impl AsRef<str>, url)`
    instead of `(Method, Url)`. The builder has `header`, `body_bytes`, `body_string`,
    `body_json`, `expect_string`, `expect_json` and `build`. It builds the protocol `HttpRequest`
    directly and sends what the std builder would send: header names lowercased (as
    `http::HeaderName` does), `header` replacing an existing one, the same `content-type`
    per `body_*`, and the URL normalised through `Url` (so `https://host` goes out as
    `https://host/`).

So **std and no_std share names and call shapes, not types**. counter_http's HTTP code
(`Http::get(url).expect_json().build().then_send(Event::Set)`, `Http::post(url)…`,
`Url::parse(..).join(..)`, `crux_http::Result<crux_http::Response<Count>>`,
`response.take_body()`, matching on `HttpError`) compiles against either. Anything that
touches a `StatusCode`, `HeaderValue` or `Mime` does not: `response.status().as_u16()` is
std-only, and `response.status()` is already a `u16` without std.

### For the RFC

1. **The subset is a second API to keep in step.** Every builder method added to the std
   `RequestBuilder` needs a decision for the no_std one. A proper version would probably
   make crux_http's public types its own (status as `u16` or a small newtype, headers as an
   ordered `Vec`), with `http` interop behind a feature, so that one API serves both. That
   is a breaking change and needs its own RFC.
2. **`http` may go no_std upstream** (the `compile_error!` says "may be added later"); if it
   does, most of the twin code can go.
3. **facet-generate-attrs** should depend on `facet` with `default-features = false`. It is
   ours, and it is a one-line change.
4. `derive_builder`'s no_std mode works on stable, despite its docs.
5. The no_std builder has no middleware. Whether `crux_http::middleware` (async-trait,
   `Box<dyn>`, `Arc`) is wanted on MCUs is the same question as crux_core's effect
   middleware (cpb-counter SPIKE_NOTES §10.6).
