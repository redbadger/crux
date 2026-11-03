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
| `memory-s140-v6.x`, `memory-s140-v7.x`, `build.rs`, `.cargo/config.toml` | linker layouts for the board's bootloader and SoftDevice (one for each S140 version), target `thumbv7em-none-eabihf` |
| `heap-probe/` | runs the core on the host, as the firmware builds it, with a counting allocator, and reports peak heap |

## Running

Everything goes through `just` (run `just --list`):

```sh
just doctor      # check the tools: the thumbv7em target, cargo-binutils, python3, curl
just check       # fmt + clippy (pedantic), firmware and heap probe
just build       # release firmware
just size        # section sizes
just heap-probe  # peak heap on the host (pointers are 8 bytes there, so it's an upper bound)
just board       # print the board's SoftDevice and its memory layout (flashes nothing)
just uf2         # target/cpb-counter.uf2, ready to copy onto the board
```

To put it on the board:

1. Plug the board in over USB with a data cable.
2. Double-press the small reset button in the middle. The NeoPixels turn green and a drive
   called `CPLAYBTBOOT` appears.
3. Run `just flash`. It reads the board's SoftDevice, builds a UF2 for the matching layout
   (downloading Microsoft's `uf2conv.py` into `target/` the first time), and copies it onto the
   drive. The board resets into the firmware.

`just flash` expects the drive at `/Volumes/CPLAYBTBOOT` on macOS and
`/media/$USER/CPLAYBTBOOT` elsewhere; set `CPB_DRIVE` to override. To go back to
CircuitPython, double-press reset and copy its UF2 onto the drive.

## SoftDevice versions

The board starts the application at the end of its SoftDevice. The firmware never enables the
SoftDevice, so only the flash origin depends on its version. Both versions work:

| SoftDevice | Cargo feature | Flash origin | Memory file |
|---|---|---|---|
| S140 6.1.1 (what boards ship with) | `sd-v6` (the default) | 0x26000 | `memory-s140-v6.x` |
| S140 7.x | `sd-v7` | 0x27000 | `memory-s140-v7.x` |

`just flash` and `just uf2` read the `SoftDevice: S140 x.y.z` line in `INFO_UF2.TXT` on the
`CPLAYBTBOOT` drive. A 6.x version selects `sd-v6` and a 7.x version selects `sd-v7`. Any other
version stops the recipe with an error. `just board` prints the detected version and layout and
does nothing else.

To choose the layout without reading the board, pass it as the argument: `just flash v7` or
`just uf2 v6`. `just build` and `just size` build `v6` by default, and `just build v7` builds
the other layout. Without `just`, build with `cargo build --release` (v6) or
`cargo build --release --no-default-features --features sd-v7` (v7). See SPIKE_NOTES §7 for
why the layout depends on the SoftDevice.
