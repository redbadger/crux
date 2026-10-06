# Counter on a Circuit Playground Bluefruit (no_std spike)

Spike: a Crux core running as `no_std` firmware on an Adafruit
[Circuit Playground Bluefruit](https://www.adafruit.com/product/4333) (nRF52840, Cortex-M4F,
1 MB flash, 256 KB RAM), with [embassy](https://embassy.dev) as the shell. It's evidence for
an RFC on `no_std` support, not mergeable code. The findings, including what had to change in
`crux_core`, are in [SPIKE_NOTES.md](./SPIKE_NOTES.md).

On the board: button A adds a NeoPixel, button B removes one (negative counts are red), the
slide switch sets the brightness, and the red LED (D13) flashes on each press. The flash is a
custom async `Delay` effect, resolved by the shell with an embassy timer.

| Path | What |
|---|---|
| `src/app.rs` | the Crux app (`no_std`, knows nothing about the board) |
| `src/main.rs` | the firmware shell: buttons, switch, NeoPixels (WS2812 over PWM), `Delay` |
| `memory.x`, `build.rs`, `.cargo/config.toml` | linker layout for the board's bootloader, target `thumbv7em-none-eabihf` |
| `heap-probe/` | runs `app.rs` on the host with a counting allocator and reports peak heap |

## Running

Everything goes through `just` (run `just --list`):

```sh
just doctor      # check the tools: the thumbv7em target, cargo-binutils, python3, curl
just check       # fmt + clippy (pedantic), firmware and heap probe
just build       # release firmware
just size        # section sizes
just heap-probe  # peak heap on the host (pointers are 8 bytes there, so it's an upper bound)
```

To put it on the board:

1. Plug the board in over USB with a data cable.
2. Double-press the small reset button in the middle. The NeoPixels turn green and a drive
   called `CPLAYBTBOOT` appears.
3. Run `just flash`. It builds a UF2 (downloading Microsoft's `uf2conv.py` into `target/` the
   first time) and copies it onto the drive. The board resets into the firmware.

`just flash` expects the drive at `/Volumes/CPLAYBTBOOT` on macOS and
`/media/$USER/CPLAYBTBOOT` elsewhere; set `CPB_DRIVE` to override. To go back to
CircuitPython, double-press reset and copy its UF2 onto the drive.

The firmware is linked at 0x26000, the end of the S140 6.1.1 SoftDevice the board ships with
(check `INFO_UF2.TXT` on the drive). A board updated to S140 7.x needs 0x27000 in both
`memory.x` and the `base` in the Justfile. See SPIKE_NOTES §7 for why.
