# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/redbadger/crux/compare/crux_cli-v0.1.1...crux_cli-v0.2.0) - 2026-03-20

### ⚠️ Breaking Changes

- The `--swift` and `--kotlin` arguments to `bindgen` now accept an optional output directory.
- At least one bindgen language (`--kotlin` or `--swift`) is now required.

```
Usage: crux bindgen [OPTIONS]

Options:
  -c, --crate-name <STRING>  Package name of the crate containing your Crux App [default: shared]
  -k, --kotlin <DIR>         Generate bindings for Kotlin at the specified path
  -s, --swift <DIR>          Generate bindings for Swift at the specified path
  -h, --help                 Print help
  -V, --version              Print version
```

### 🚀 Features

- **`BindgenArgsBuilder`**: Added a builder API for constructing bindgen arguments programmatically, improving library API ergonomics.

### ⚙️ Miscellaneous Tasks

- Update to `facet_generate` 0.15, `facet` 0.31, and other Rust dependencies.
- Update Rust dependencies.

## [0.1.1](https://github.com/redbadger/crux/releases/tag/crux_cli-v0.1.1) - 2025-08-19

Added support for calling bindgen through the library API.

## [0.1.0](https://github.com/redbadger/crux/releases/tag/crux_cli-v0.1.0) - 2025-05-27

Initial release.