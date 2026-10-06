# Firmware

The Circuit Playground Bluefruit side of cpb-counter-http: embassy on the nRF52840, with
[TrouBLE](https://github.com/embassy-rs/trouble) as the BLE host and Nordic's SoftDevice
Controller ([`nrf-sdc`](https://github.com/alexmoon/nrf-sdc)) linked into the image. The
board's own SoftDevice (S140 6.1.1) stays resident but is never enabled, so the board needs
no bootloader or SoftDevice update.

## Binaries

| Binary | What it's for |
|---|---|
| `cpb-counter-http` (default) | The app: counter_http's Crux core (`../app`) with its HTTP and SSE effects sent over BLE |
| `link-check` | Bring-up of the whole link without Crux: on connect it sends a raw GET through the gateway |
| `ble-probe` | Bring-up of the radio alone: advertises and echoes writes back as notifications |

```sh
just build                 # all three
just size                  # section sizes (just size link-check, …)
just check                 # fmt + clippy (pedantic)
just flash                 # double-press reset first; or: just flash link-check
```

`just flash` builds a UF2 at 0x26000 (downloading Microsoft's `uf2conv.py` into `target/` the
first time) and copies it to `CPLAYBTBOOT`. That's `/Volumes/CPLAYBTBOOT` on macOS and
`/media/$USER/CPLAYBTBOOT` elsewhere; set `CPB_DRIVE` to override.

## Reading the pixels

There's no debug probe, so the NeoPixels are the debug output.

- **`cpb-counter-http`**: one dim blue pixel while waiting for the gateway. Then the count:
  green for positive, red for negative, dimmed (with D13 lit) while a change is pending.
  Alternating red and blue means the last request failed. All red means the radio stack failed to start.
- **`link-check`**: pixel 0 is dim blue while advertising and cyan once connected. Pixel 1 goes
  amber while the GET is in flight, then green for a 2xx answer, red otherwise. Button A
  repeats the GET, and button B opens the SSE stream; pixel 2 flashes violet for each chunk.
- **`ble-probe`**: pixel 0 is dim blue while advertising and green once connected. Pixel 1
  flashes white for each echoed write. Test it from Chrome with `tools/ble-probe.html`
  (open the file directly; `file://` counts as a secure context).

## Layout

| Path | What |
|---|---|
| `src/main.rs` | the app's shell: `Core<Counter>`, debounced buttons, the link, `Delay` timers |
| `src/link.rs` | radio set-up (MPSL, SDC), the GATT service, framing; channels to the shell |
| `src/neopixel.rs` | WS2812 over PWM |
| `src/bin/` | `link-check` and `ble-probe` |
| `memory.x` | flash at 0x26000 and RAM at 0x20006000, leaving room for the bootloader's MBR and SoftDevice |

Board details that matter here:
- The CPB has no 32 kHz crystal, so the radio's low-frequency clock runs from the RC oscillator.
- MPSL owns RTC0, TIMER0, TEMP, RADIO, CLOCK, EGU0, the RNG and PPI channels 17–31.
- P0.06 must be held low, or the NeoPixels have no power.

`../SPIKE_NOTES.md` has the details and the measurements.
