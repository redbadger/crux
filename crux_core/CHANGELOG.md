# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.3](https://github.com/redbadger/crux/compare/crux_core-v0.7.2...crux_core-v0.7.3) - 2024-02-02

### Fixed
- fix doc test deps

### Other
- remove Debug bound in typegen
- update `crux_time` to not use generic response type + overall deps
- Make rust fmt happy
- Export crux_macros from crux_core and change docs
- Use options helper and erase serde inside the bridge
- Fix doctest
- Rename a bunch of things to better reflect how they work
- fun with erased_serde
- Tidy doc comments
- Better names, mod structure, and documentation with plentiful warnings
- Fix doctests
- Add a JSON bridge test for the pluggable Bridge serializers
- Merge serialization traits into one
- Initial cut of allowing bridge to use different serialization
- Merge branch 'redbadger:master' into master

## [0.7.2](https://github.com/redbadger/crux/compare/crux_core-v0.7.1...crux_core-v0.7.2) - 2024-01-26

### Fixed
- fix clippy lints

### Other
- Introduce a Compose capability which allows composition of other capabilities
- Introduce `Never` type for capabilities that don't request effects
- Effect derive macro now allows skipping variants (to support `Never` operations)
- Make render capability is now Clone to suport composition
- remove uuid unused wasm-bindgen feature flag

## [0.7.1](https://github.com/redbadger/crux/compare/crux_core-v0.7.0...crux_core-v0.7.1) - 2024-01-11

### Other
- update deps for Rust, Web, iOS and Android
- update examples to crux_core 0.7

## [0.7.0](https://github.com/redbadger/crux/compare/crux_core-v0.6.5...crux_core-v0.7.0) - 2023-12-03

### Fixed
- fix doc tests

### Other
- improve typegen error handling

## [0.6.5](https://github.com/redbadger/crux/compare/crux_core-v0.6.4...crux_core-v0.6.5) - 2023-11-29

### Other
- root deps
- rustfmt
- full error message

## [0.6.4](https://github.com/redbadger/crux/compare/crux_core-v0.6.3...crux_core-v0.6.4) - 2023-10-25

### Other
- update deps
- update leptos examples to remove Scope
- deps + tweaks
- avoid unnecessary coercion
- Remove existing generated java files before generating the new set
- deps
- deps
