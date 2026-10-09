# Counter (HTTP) on a Circuit Playground Bluefruit, over BLE (no_std spike)

Spike: the counter-http Crux core ([`../shared`](../shared)), the same one the iOS, Android
and web shells use, running as `no_std` firmware on an Adafruit
[Circuit Playground Bluefruit](https://www.adafruit.com/product/4333) (nRF52840), with
[embassy](https://embassy.dev) as the shell. The board has no IP stack, so the core's HTTP
requests and Server-Sent Events go over Bluetooth LE to a page in Chrome, which is itself a
Crux app and performs them with `fetch`. It's evidence for an RFC on `no_std` support, not
mergeable code. Findings are in [SPIKE_NOTES.md](./SPIKE_NOTES.md).

```
 Circuit Playground Bluefruit                       Chrome                       internet
┌──────────────────────────────┐   BLE GATT   ┌────────────────────────┐  fetch  ┌───────────────────┐
│ ../shared (Crux core, no_std)│  tx: notify  │ gateway core (Crux)    │ ──────▶ │ crux-counter      │
│   crux_http, SSE, crux_time  │ ───────────▶ │   forwards HttpRequest │         │   .fly.dev        │
│ this shell (embassy)         │  rx: write   │   and SseRequest       │ ◀────── │                   │
│   buttons, NeoPixels, link   │ ◀─────────── │ Leptos + Web Bluetooth │         │                   │
└──────────────────────────────┘              └────────────────────────┘         └───────────────────┘
```

The firmware is its own Cargo workspace, excluded from counter-http's (`exclude = ["cpb"]` in
`../Cargo.toml`). It builds for another target with its own release profile, and depends on
`shared` with `default-features = false`, so feature unification with the std shells can't
pull std back in. It turns on `crux_core/critical-section`; `nrf-mpsl` provides the
critical-section implementation.

Like the other shells, this one handles the core's effects:

- `Http`: sent to the gateway as `ToGateway::Http`, and resolved with its answer.
- `ServerSentEvents`: the core's own SSE capability. The shell copies the request's url into
  the wire copy (`ble_protocol::SseRequest`), and resolves each `SseChunk` / `SseDone` from
  the gateway as `SseResponse::Chunk` / `Done`.
- `TimeNotifyAfter` / `TimeClear` (`crux_time`, which paces reopening the event stream): a
  task of their own with embassy timers (`src/delay.rs`), answered as `crux_time`'s shipped
  shell handlers answer them.
- `Render`: draws the view on the NeoPixels.

When the gateway connects the shell sends `Get` and `StartWatch`, as the other shells do at
start-up. When it goes away the shell fails the requests in flight and ends the stream; the
core reopens the stream after a wait (1 s, doubling to 30 s), and keeps trying until the
gateway is back.

On the board: button A sends `Increment` and button B sends `Decrement`; the slide switch sets
the brightness. The NeoPixels show the view's `value` as an odometer: past 10 the ring wraps,
each lap of ten filling in a new colour over the last (positive: green, cyan, blue; negative:
red, orange, magenta; then round again). While the count is waiting for the server
(`confirmed` is false) the pixels are dimmed and D13 is lit. One dim blue pixel means "waiting
for the gateway", alternating red and blue means the view has an `error` (the last request
failed), and all red means the radio stack failed to start. Brightness, the link indicator
and the colours are presentation, so they live in the shell.

| Path | What |
|---|---|
| `src/main.rs` | the shell: `Core<Counter>`, debounced buttons, the switch, rendering, the link's effects |
| `src/delay.rs` | the timer task: embassy timers, resolved with `Core::resolve`, effects queued back to the shell |
| `src/link.rs` | radio set-up (MPSL, SDC), the GATT service, framing; channels to the shell |
| `src/neopixel.rs` | WS2812 over PWM |
| `src/bin/` | `link-check` and `ble-probe`, bring-up binaries without the core |
| `memory.x`, `build.rs`, `.cargo/config.toml` | flash at 0x26000 and RAM at 0x20006000, leaving room for the bootloader's MBR and SoftDevice; target `thumbv7em-none-eabihf` |
| `heap-probe/` | runs the core on the host, as the firmware builds it, with a counting allocator, and reports peak heap |
| `tools/ble-probe.html` | a Web Bluetooth page for `ble-probe` |

The app-agnostic BLE parts live in `examples_support/`:

| Directory | What | Builds for |
|---|---|---|
| [`ble_protocol/`](../../../examples_support/ble_protocol/) | messages over BLE: `ToGateway`/`ToDevice` carrying crux_http's protocol types, framing, UUIDs, and a wire copy of `SseRequest` | thumbv7em, wasm, host |
| [`ble_gateway/`](../../../examples_support/ble_gateway/) | the Chrome gateway: `core` (a Crux app) and `web` (Leptos) | wasm, host (tests) |

## Running

Everything goes through `just` (`just --list`). You need Chrome (for Web Bluetooth) and a
Circuit Playground Bluefruit.

```sh
just doctor           # the tools, here and for the gateway
just check            # fmt + clippy (pedantic, nursery) for the firmware and the heap probe
just build            # the three binaries: cpb-counter-http, link-check, ble-probe
just size             # section sizes (just size link-check, …)
just heap-probe       # peak heap on the host (8-byte pointers, so an upper bound)
just heap-probe-wasm  # the same as wasm32-wasip1 under Node: 4-byte pointers, as on the board
just uf2              # target/cpb-counter-http.uf2, ready to copy onto the board
```

The core's tests are in `../shared` (`cargo nextest run -p shared` in `..`); the protocol's and
gateway's are in theirs.

Then, to run it end to end:

1. `just serve` starts the gateway (`examples_support/ble_gateway`) at
   http://127.0.0.1:8080. Leave it running, and open that page in Chrome.
2. Double-press the board's reset button. When the `CPLAYBTBOOT` drive appears, run
   `just flash` in another terminal. One dim blue pixel means the board is advertising.
3. In the gateway page, click **Connect** and choose "CPB Counter". The board shows the shared
   count from crux-counter.fly.dev, and the gateway logs each request it forwards.

`just flash` builds a UF2 at 0x26000 (downloading Microsoft's `uf2conv.py` into `target/` the
first time) and copies it to `CPLAYBTBOOT`. That's `/Volumes/CPLAYBTBOOT` on macOS and
`/media/$USER/CPLAYBTBOOT` elsewhere; set `CPB_DRIVE` to override.

Changes made from any other counter-http client (for example `../web-leptos`) appear on the
board, and presses on the board appear there.

## Bring-up binaries

There's no debug probe, so the NeoPixels are the debug output. `just flash link-check` and
`just flash ble-probe` flash these instead of the shell:

- **`link-check`**: the whole link without Crux. Pixel 0 is dim blue while advertising and
  cyan once connected. Pixel 1 goes amber while a raw GET is in flight, then green for a 2xx
  answer, red otherwise. Button A repeats the GET, and button B opens the SSE stream; pixel 2
  flashes violet for each chunk.
- **`ble-probe`**: the radio alone. Pixel 0 is dim blue while advertising and green once
  connected. Pixel 1 flashes white for each echoed write. Test it from Chrome with
  `tools/ble-probe.html` (open the file directly; `file://` counts as a secure context).

Board details that matter here:
- The radio is TrouBLE (the BLE host) with Nordic's SoftDevice Controller (`nrf-sdc`) linked
  into the image. The board's own SoftDevice (S140 6.1.1) stays resident but is never
  enabled, so the board needs no bootloader or SoftDevice update.
- The CPB has no 32 kHz crystal, so the radio's low-frequency clock runs from the RC oscillator.
- MPSL owns RTC0, TIMER0, TEMP, RADIO, CLOCK, EGU0, the RNG and PPI channels 17–31.
- P0.06 must be held low, or the NeoPixels have no power.
