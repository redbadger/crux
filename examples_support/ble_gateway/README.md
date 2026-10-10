# BLE gateway

A page in Chrome that connects to a Crux device over Web Bluetooth (for now, the Circuit
Playground Bluefruit in [`examples/counter-http/cpb`](../../examples/counter-http/cpb/)) and performs its
HTTP requests and Server-Sent Events streams. It speaks [`ble_protocol`](../ble_protocol/) and
knows nothing about the device's app. It's a Crux app too, and a workspace of its own:

| Crate | What |
|---|---|
| `core/` | `gateway-core`: reassembles the device's messages, re-emits each `HttpRequest` / `SseRequest` as an effect (SSE as its own `SseStream`, so the shell can say whether a stream was closed or failed), and frames the answers back. It works as a generic proxy for one device |
| `web/` | `gateway-web`: the Leptos shell. Web Bluetooth (`src/ble.rs`), `fetch` for HTTP and SSE (adapted from counter-http's Leptos shell), and a log of what it forwards, each line stamped with the time the shell first showed it |

```sh
just doctor   # trunk and the wasm32 target
just serve    # http://127.0.0.1:8080; open it in Chrome
just test     # the core's tests
just check    # fmt + clippy (pedantic), core and the wasm shell
```

Click **Connect** and choose the device. The chooser lists devices advertising the gateway
service (`ble_protocol::SERVICE_UUID`), by the name each firmware gives itself (e.g.
"CPB Counter").
Chrome remembers the device, so reconnecting later needs no chooser. Web Bluetooth needs
Chrome (or another Chromium) and a secure context; `localhost` counts.

Points worth knowing:
- Chrome allows one GATT operation at a time, so the core queues its writes and sends the
  next only when the previous one has finished.
- The core writes in 20-byte chunks, because Chrome doesn't reveal the negotiated MTU.
- When the device disconnects, the core aborts its open SSE streams. The shell then stops
  reading the response body, which closes the fetch.
- An SSE stream ends on the device the same way (`SseDone`) whether the server closed it or
  it failed (fetch rejected, a non-2xx status, or a body read error); only the log tells them
  apart (`SSE closed by the server` / `SSE failed: …`). The device reopens it either way.
- Web Bluetooth is behind web-sys's unstable APIs: `.cargo/config.toml` sets
  `--cfg=web_sys_unstable_apis` for wasm builds.
