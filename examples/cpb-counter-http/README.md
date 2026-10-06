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
| `protocol/` | messages over BLE: `ToGateway`/`ToDevice` carrying crux_http's protocol types, framing, UUIDs, `SseRequest` | thumbv7em, wasm, host |
| `firmware/` | the board: `cpb-counter-http` (the app), plus `ble-probe` and `link-check` (bring-up) | thumbv7em |
| `gateway/` | the Chrome gateway: `core` (a Crux app) and `web` (Leptos) | wasm, host (tests) |
| `heap-probe/` | runs the app as the shell does and reports peak heap | host, wasm32-wasip1 |

On the board: button A increments and button B decrements; the slide switch sets the
brightness. The NeoPixels show the count (green positive, red negative, up to 10). D13 is lit
while a change is waiting for the server. One dim blue pixel means "waiting for the
gateway", and alternating red and blue means the last request failed.

## Running

Needs Chrome (Web Bluetooth), `trunk`, and for the firmware
`rustup target add thumbv7em-none-eabihf`, `rustup component add llvm-tools` and
`cargo install cargo-binutils`.

Tests: `cargo test` here (app, protocol), and `cargo test -p gateway-core` in `gateway/`.

Gateway: `cd gateway/web && trunk serve`, then open http://127.0.0.1:8080.

Firmware:

```sh
cd firmware
cargo build --release
cargo objcopy --release --bin cpb-counter-http -- -O binary target/cpb-counter-http.bin
# uf2conv.py and uf2families.json, from github.com/microsoft/uf2 utils/ (not committed)
python3 uf2conv.py target/cpb-counter-http.bin -c -b 0x26000 -f 0xADA52840 -o target/cpb-counter-http.uf2
```

Double-press the board's reset button. When the `CPLAYBTBOOT` drive appears, copy the
`.uf2` onto it (0x26000 assumes the S140 6.x SoftDevice the board ships with; see
`../cpb-counter/SPIKE_NOTES.md` §7). Then click **Connect** in the gateway and pick "CPB Counter".
