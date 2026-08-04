# Handling `crux_http` rejections

When a server rejects a request — a 4xx or a 5xx — `crux_http` does **not** hand your
app a `Response` with that status on it. It converts the rejection into an
`Err(HttpError::Http { .. })`, keeping the status, the headers and the body, and sends
*that* to your update function.

So an `Ok(Response)` always means the server did not reject the request, and
`Response::status()` is always a 1xx, 2xx or 3xx.

---

## The shape to write

This is the trap the invariant exists to close:

```rust
// WRONG — the `else` branch is dead code
match result {
    Ok(mut response) => {
        if response.status().is_success() {
            // …
        } else {
            // unreachable: a 4xx never arrives as Ok(Response)
            show(error_reason(&mut response));
        }
    }
    // …so this is where a 409 lands, and `to_string()` is only "HTTP error 409: 409 Conflict"
    Err(error) => show(&error.to_string()),
}
```

Everything the server said about *why* it refused is on the error, so read it there:

```rust
match result {
    // no status check here — this arm cannot see a 4xx or 5xx
    Ok(response) => saved(response.body()),
    Err(error) => {
        let message = error
            .body_json::<serde_json::Value>()
            .ok()
            .and_then(|body| body["error"].as_str().map(str::to_string))
            .unwrap_or_else(|| error.to_string());
        show(&message);
    }
}
```

---

## Reading a rejection

`HttpError` has accessors for everything the rejected response carried, so you never
need to destructure the variant:

| Accessor | What you get |
| --- | --- |
| `code()` | The status the server rejected with, e.g. `Some(409)` |
| `body()` | The raw body bytes the server sent, if any |
| `body_json::<T>()` | That body deserialized — your own error envelope, an RFC 7807 `problem+json` struct, or `serde_json::Value` |
| `header(name)` | One header, looked up case-insensitively |
| `content_type()` | The parsed `Content-Type`, if the response declared one |
| `headers()` | The whole `HeaderMap`, for multi-value headers or logging |

Body decoding is skipped for an error status, so the raw error body survives even when
the request was built with `.expect_json::<T>()` — an error envelope that doesn't match
`T` still reaches you intact.

`HttpError::Http` is raised for a rejection and nothing else, so `code().is_some()` is the
test for "the server said no". The crate's own failures have their own variants —
`Json` (a body that wouldn't deserialize), `BodyAlreadyTaken` (you read the body twice),
`InvalidStatusCode` (the shell sent a status that isn't valid HTTP) — as do the shell's
transport failures (`Url`, `Io`, `Timeout`). None of them carry a status, because no server
chose one.

The headers matter more than they first appear, because some of what an app needs in
order to *act* on a rejection is only there:

```rust
// back off politely instead of hammering the server
if let Some(retry_after) = error.header("Retry-After") { … }

// silent token refresh, or send the user back to the login screen?
if error.code() == Some(401) {
    match error.header("WWW-Authenticate") { … }
}

// is this body an error envelope, or a proxy's HTML page?
match error.content_type() { … }
```

```admonish note
`HttpError::Http` is `#[non_exhaustive]`: only `crux_http` constructs it, so what a
rejection carries can grow without breaking your code. Match it with `{ code, .. }` and
reach for the accessors above for the rest.
```

---

## Testing a rejection

There are exactly two values a feature can receive, and `crux_http::testing` has one
builder for each:

- `ResponseBuilder` — the `Ok(Response)` of a successful exchange. It **panics** if you
  give it a 4xx or 5xx, because no app can ever receive one.
- `rejection(status, body)` — the `Err` of a rejection. Use `rejection_from(response)`
  when the rejection's headers are what your feature acts on.

Both run the same conversion a real shell response takes, so what they produce is what
your app is really handed.

```rust
// before — asserts a state the app can never observe, so the test passes
// while the code it covers is dead
let result = Ok(ResponseBuilder::with_status(409).body(body).build());

// after — what the app really receives
let result = crux_http::testing::rejection(409, body);

// …or, when the headers are the point
let result = crux_http::testing::rejection_from(
    HttpResponse::status(503)
        .header("retry-after", "120")
        .body(b"maintenance".to_vec())
        .build(),
);
```

If you have tests that assert on a rejection message and they pass today, check which
value they build. A test that fabricates `Ok(response_with_409)` is asserting the dead
`else` branch above, and will happily stay green while your users see
`"HTTP error 409: 409 Conflict"`.

---

## See also

- [Migrating `crux_http` to native `http` types](./migrate-crux-http.md), for the 0.19
  change from `http-types` to the `http` crate.
