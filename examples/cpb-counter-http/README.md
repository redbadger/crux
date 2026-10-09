# Counter (HTTP) on a Circuit Playground Bluefruit, over BLE

Spike: [`counter-http`](../counter-http/)'s Crux app running as `no_std` firmware on an
Adafruit Circuit Playground Bluefruit (nRF52840). The board has no IP stack, so its HTTP
requests and Server-Sent Events go over Bluetooth LE to a page in Chrome, which is itself a
Crux app and performs them with `fetch`. Findings are in [SPIKE_NOTES.md](./SPIKE_NOTES.md).

```
 Circuit Playground Bluefruit                       Chrome                       internet
┌──────────────────────────────┐   BLE GATT   ┌────────────────────────┐  fetch  ┌───────────────────┐
│ app (Crux core, no_std)      │  tx: notify  │ gateway core (Crux)    │ ──────▶ │ crux-counter      │
│   crux_http, SSE             │ ───────────▶ │   forwards HttpRequest │         │   .fly.dev        │
│ firmware shell (embassy)     │  rx: write   │   and SseRequest       │ ◀────── │                   │
│   buttons, NeoPixels, link   │ ◀─────────── │ Leptos + Web Bluetooth │         │                   │
└──────────────────────────────┘              └────────────────────────┘         └───────────────────┘
```

| Directory | What | Builds for |
|---|---|---|
| `app/` | counter_http's app, `no_std`. Its HTTP code is unchanged | thumbv7em, host (tests) |
| `firmware/` | the board: `cpb-counter-http` (the app), plus `ble-probe` and `link-check` (bring-up) | thumbv7em |
| `heap-probe/` | runs the app as the shell does and reports peak heap | host, wasm32-wasip1 |

Each directory has its own README. The app-agnostic BLE parts live in `examples_support/`:

| Directory | What | Builds for |
|---|---|---|
| [`ble_protocol/`](../../examples_support/ble_protocol/) | messages over BLE: `ToGateway`/`ToDevice` carrying crux_http's protocol types, framing, UUIDs, and a wire copy of `SseRequest` | thumbv7em, wasm, host |
| [`ble_gateway/`](../../examples_support/ble_gateway/) | the Chrome gateway: `core` (a Crux app) and `web` (Leptos) | wasm, host (tests) |

On the board: button A increments and button B decrements; the slide switch sets the
brightness. The NeoPixels show the count (green positive, red negative); past 10 the ring
wraps like an odometer, each lap of ten filling in a new colour over the last (positive:
green, cyan, blue; negative: red, orange, magenta; then round again). D13 is lit
while a change is waiting for the server. One dim blue pixel means "waiting for the
gateway", and alternating red and blue means the last request failed. If the network drops,
the board reopens its SSE stream by itself, backing off from 1 s to 30 s.

## Running

Everything goes through `just` (`just --list` here, in `firmware/`, and in
`examples_support/ble_protocol/` and `examples_support/ble_gateway/`).
You need Chrome (for Web Bluetooth) and a Circuit Playground Bluefruit.

```sh
just doctor           # check the tools for all the parts
just test             # app tests on the host (the protocol's and gateway's: `just test` in theirs)
just check            # fmt + clippy (pedantic) here and in firmware/, including thumbv7em
just heap-probe-wasm  # the 32-bit heap numbers in SPIKE_NOTES (needs Node)
```

Then, to run it end to end:

1. `just serve` starts the gateway (`examples_support/ble_gateway`) at http://127.0.0.1:8080. Leave it running, and open
   that page in Chrome.
2. Double-press the board's reset button. When the `CPLAYBTBOOT` drive appears, run
   `just flash` in another terminal. One dim blue pixel means the board is advertising.
3. In the gateway page, click **Connect** and choose "CPB Counter". The board shows the shared
   count from crux-counter.fly.dev, and the gateway logs each request it forwards.

Changes made from any other counter_http client (for example
`examples/counter-http/web-leptos`) appear on the board, and presses on the board appear there.

`just flash link-check` and `just flash ble-probe` flash the bring-up binaries instead; see
[firmware/README.md](./firmware/README.md).
