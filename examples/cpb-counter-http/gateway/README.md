# Gateway

A page in Chrome that connects to the Circuit Playground Bluefruit over Web Bluetooth and
performs its HTTP requests and Server-Sent Events streams. It's a Crux app too:

| Crate | What |
|---|---|
| `core/` | `gateway-core`: reassembles the device's messages, re-emits each `HttpRequest` / `SseRequest` as an effect, and frames the answers back. It knows nothing about the counter, so it works as a generic proxy for one device |
| `web/` | `gateway-web`: the Leptos shell. Web Bluetooth (`src/ble.rs`), `fetch` for HTTP and SSE (adapted from counter-http's Leptos shell), and a log of what it forwards |

```sh
just doctor   # trunk and the wasm32 target
just serve    # http://127.0.0.1:8080; open it in Chrome
just test     # the core's tests
just check    # fmt + clippy (pedantic), core and the wasm shell
```

Click **Connect** and choose "CPB Counter". Chrome remembers the device, so reconnecting
later needs no chooser. Web Bluetooth needs Chrome (or another Chromium) and a secure
context; `localhost` counts.

Points worth knowing:
- Chrome allows one GATT operation at a time, so the core queues its writes and sends the
  next only when the previous one has finished.
- The core writes in 20-byte chunks, because Chrome doesn't reveal the negotiated MTU.
- When the device disconnects, the core aborts its open SSE streams. The shell then stops
  reading the response body, which closes the fetch.
- Web Bluetooth is behind web-sys's unstable APIs: `.cargo/config.toml` sets
  `--cfg=web_sys_unstable_apis` for wasm builds.
