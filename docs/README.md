# Documentation Guide

This is built with [mdBook](https://github.com/rust-lang/mdBook).

From `docs/`, the main entry points are:

```sh
just doctor
just dev
just serve
```

`just doctor` checks that the required tooling is installed.

`just dev` runs `mdbook build`, including the configured preprocessors and link
checking.

`just serve` starts the local docs server.

## Diagrams

The script `diagrams/generate.rs` makes the SVG diagrams in `src/`
(`command_overview.svg`, `architecture.svg`, `crux.svg`, `shell_apple.svg`,
`shell_android.svg`, `shell_react.svg` and `shell_leptos.svg`). Do not edit
these files directly. Change the script, then run `just diagrams`.

The script is a single-file Rust script with no dependencies. Cargo runs it
only on a nightly toolchain, so install one with `rustup toolchain install nightly`.
