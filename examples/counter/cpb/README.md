# Counter on a Circuit Playground Bluefruit (no_std spike)

Spike: the counter's Crux core ([`../shared`](../shared)), the same one the iOS, Android and
web shells use, running as `no_std` firmware on an Adafruit
[Circuit Playground Bluefruit](https://www.adafruit.com/product/4333) (nRF52840, Cortex-M4F,
1 MB flash, 256 KB RAM), with [embassy](https://embassy.dev) as the shell. It's evidence for
an RFC on `no_std` support, not mergeable code. The findings, including what had to change in
`crux_core`, are in [SPIKE_NOTES.md](./SPIKE_NOTES.md).

On the board: button A sends `Increment` and button B sends `Decrement`. On each `Render` the
shell draws the view's `value` on the ten NeoPixels as an odometer: the core's count is
unbounded, so each lap of ten fills a new colour over the last (green, cyan, blue counting up;
red, orange, magenta counting down). The slide switch sets the brightness and the red LED (D13) flashes on each
press; both are presentation, so they live in the shell and the core never hears about them.

The firmware is its own Cargo workspace, excluded from the counter's (`exclude = ["cpb"]` in
`../Cargo.toml`). It builds for another target with its own release profile, and depends on
`shared` with `default-features = false`, so feature unification with the std shells can't
pull std back in.

| Path | What |
|---|---|
| `src/main.rs` | the shell: buttons, switch, NeoPixels (WS2812 over PWM), LED flash |
| `memory.x`, `build.rs`, `.cargo/config.toml` | linker layout for the board's bootloader, target `thumbv7em-none-eabihf` |
| `heap-probe/` | runs the core on the host, as the firmware builds it, with a counting allocator, and reports peak heap |

## Running

Everything goes through `just` (run `just --list`):

```sh
just doctor      # check the tools: the thumbv7em target, cargo-binutils, python3, curl
just check       # fmt + clippy (pedantic), firmware and heap probe
just build       # release firmware
just size        # section sizes
just heap-probe  # peak heap on the host (pointers are 8 bytes there, so it's an upper bound)
just uf2         # target/cpb-counter.uf2, ready to copy onto the board
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
