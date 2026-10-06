# App

counter_http's Crux app as a `no_std` crate, for the Circuit Playground Bluefruit. Its
HTTP code is counter_http's unchanged, compiled against crux_http's `no_std` subset.
The header of `src/lib.rs` lists what differs from counter_http, and why.

- `src/lib.rs`: the app. Its view is ten NeoPixels and the red LED.
- `src/sse.rs`: the `ServerSentEvents` command, with a small parser that buffers across
  chunks.
- `src/tests.rs`: counter_http's tests, ported, plus tests for the board-specific behaviour.

`cargo test` runs on the host; `just check` one level up also builds it for thumbv7em.
