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

The script `diagrams/generate.py` makes the SVG diagrams in `src/`
(`command_overview.svg`, `architecture.svg` and `crux.svg`). Do not edit these
files directly. Change the script, then run `just diagrams`.
