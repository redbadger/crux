# Migrating `crux_http` to native `http` types

From the next release, `crux_http::http` re-exports the upstream
[`http`](https://docs.rs/http) crate (v1.4) instead of the internal
`http-types-red-badger-temporary-fork`. This is a **breaking change** for code
that used `http-types`-specific names, but most apps need little or no
adjustment.

---

## Quick checklist

Most apps using `crux_http` through its high-level API (`Http::get(…)`,
`RequestBuilder`, `Response<T>`) will compile unchanged. Work through the
sections below only for the patterns that apply to your code.

---

## Method variants renamed

`http_types::Method` used UpperCamelCase enum variants. `http::Method` uses
`SCREAMING_SNAKE_CASE` associated constants.

```rust
// Before
use crux_http::Method;
Http::request(Method::Get,  url)
Http::request(Method::Post, url)

// After
use crux_http::Method;  // still the same import
Http::request(Method::GET,  url)
Http::request(Method::POST, url)
```

---

## Status code names and `HttpError::Http.code` is now `u16`

`http_types::StatusCode` used UpperCamelCase variants (`StatusCode::Unauthorized`).
`http::StatusCode` uses `SCREAMING_SNAKE_CASE` constants (`StatusCode::UNAUTHORIZED`).

In addition, the `code` field inside `HttpError::Http { code, .. }` changed
from `http_types::StatusCode` to a plain `u16`.

```rust
// Before
match err {
    HttpError::Http { code, .. }
        if code == crux_http::http::StatusCode::Unauthorized => { … }
    _ => { … }
}

// After — compare against the u16 directly …
match err {
    HttpError::Http { code, .. } if code == 401 => { … }
    _ => { … }
}

// … or via http::StatusCode
use crux_http::http::StatusCode;
match err {
    HttpError::Http { code, .. }
        if code == StatusCode::UNAUTHORIZED.as_u16() => { … }
    _ => { … }
}
```

---

## MIME type constants

`http_types` exposed extra MIME constants like `mime::JSON` and `mime::HTML`.
The standard `mime` crate uses longer names. `crux_http::mime` is now a direct
re-export of the `mime` crate.

```rust
// Before
use crux_http::http::mime;
request_builder.content_type(mime::JSON)
request_builder.content_type(mime::HTML)

// After
use crux_http::mime;
request_builder.content_type(mime::APPLICATION_JSON)
request_builder.content_type(mime::TEXT_HTML)
```

---

## `crux_http::http::Body` / `Headers` / `Version`

These types came from `http_types`. The replacements:

| Old | New |
| --- | --- |
| `crux_http::http::Body` (async, streaming) | `crux_http::Body` (sync, in-memory) |
| `crux_http::http::Headers` | `http::HeaderMap` |
| `crux_http::http::Version` | `http::Version` |

The new `crux_http::Body` is always in-memory (`Vec<u8>` backed). The
streaming / `AsyncRead` interface of `http_types::Body` is not carried over.

**If you need to stream a large or chunked HTTP response**, the correct Crux
pattern is a dedicated streaming capability — not `AsyncRead` on `ResponseAsync`.
`AsyncRead` was a leaky abstraction that pushed network I/O mechanics into the
core; the streaming capability pattern keeps the boundary clean.

The `examples/counter-http/shared/src/sse.rs` file shows exactly this pattern
for Server-Sent Events, but the same skeleton works for any chunked HTTP body:

```rust
// 1. Define the protocol
#[derive(Facet, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct StreamingHttpRequest { pub url: String }

#[derive(Facet, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum StreamingHttpResponse {
    Chunk(Vec<u8>),
    Done,
}

impl Operation for StreamingHttpRequest {
    type Output = StreamingHttpResponse;
}

// 2. Build a StreamBuilder capability method
pub fn stream_get<Effect, Event>(
    url: impl Into<String>,
) -> StreamBuilder<Effect, Event, impl Stream<Item = Vec<u8>>>
where
    Effect: From<Request<StreamingHttpRequest>> + Send + 'static,
    Event: Send + 'static,
{
    let url = url.into();
    StreamBuilder::new(|ctx| {
        ctx.stream_from_shell(StreamingHttpRequest { url })
            .take_while(|r| future::ready(!matches!(r, StreamingHttpResponse::Done)))
            .map(|r| match r {
                StreamingHttpResponse::Chunk(bytes) => bytes,
                StreamingHttpResponse::Done => unreachable!(),
            })
    })
}
```

The shell sends `Chunk(bytes)` for each network chunk and `Done` at EOF. The
core processes the resulting `Stream<Item = Vec<u8>>` with normal async stream
combinators. This pattern is clean, avoids any I/O in the core, and no `AsyncRead` is required.

---

## `http-compat` feature removed

The `http-compat` feature previously provided opt-in `TryFrom` / `TryInto`
conversions between `crux_http` types and the `http` crate's types. Those
conversions are now **built in and unconditional**:

```rust
// From http::Request<Body> to crux_http::Request — always available
let req: crux_http::Request = http_request.into();

// From crux_http::Response<T> to http::Response<T> — always available
let http_resp = http::Response::<Vec<u8>>::try_from(crux_response)?;
```

Remove the feature flag from your `Cargo.toml`:

```toml
# Before
crux_http = { version = "…", features = ["http-compat"] }

# After
crux_http = { version = "…" }
```

---

## Interoperating with `http_types` (opt-in)

If you have middleware or shell code that still uses `http_types::Request` or
`http_types::Response` and needs to pass them into `crux_http`, enable the new
`http-types` feature:

```toml
crux_http = { version = "…", features = ["http-types"] }
```

This provides `From<http_types::Request> for crux_http::Request`,
`From<http_types::Response> for crux_http::ResponseAsync`, and the reverse
directions. It also re-exports the `http_types` crate as `crux_http::http_types`
for convenience.

> **Note**: The `http_types::Body::into_bytes()` method is `async`, so converting
> a request *with a body* from `http_types::Request` to `crux_http::Request` will
> leave the body empty. Set `crux_http::Request::set_body(…)` separately after
> the conversion if needed.

---

## Getting multiple header values

The previous `http_types::HeaderValues` iterator is replaced by
`http::header::GetAll`. Use the new `header_all` method when a header can
appear more than once:

```rust
// Before (http_types returned HeaderValues with .iter())
let values: Vec<String> = response
    .header("link")
    .unwrap()
    .iter()
    .map(|v| v.to_string())
    .collect();

// After
let values: Vec<String> = response
    .header_all("link")
    .iter()
    .map(|v| v.to_str().unwrap_or("").to_string())
    .collect();
```

For a single value `response.header("name")` still works and returns
`Option<&http::HeaderValue>`.
