# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/redbadger/crux/compare/crux_kv-v0.1.10...crux_kv-v0.2.0) - 2024-05-14

### Breaking Changes

This is a breaking change release. We've added `delete` and `exists` operations and renamed `read` to `get` and `write` to `set`, amongst other renames and type changes. We've also improved error handling and tests. See https://github.com/redbadger/crux/pull/227 for more information. However, there are sadly still no atomic or batch operations, which will follow in a future release.

### Other

- deps
- Merge branch 'master' into relax-callback-bounds
- reduce nesting in kv output enums
- use enums instead of Result and Option for FFI types in crux_kv
- take owned values as cloning anyway
- move tests from integration to unit
- add exists, and update tests
- add delete to KeyValue, and move towards wasi-kv

## [0.1.10](https://github.com/redbadger/crux/compare/crux_kv-v0.1.9...crux_kv-v0.1.10) - 2024-03-24

### Other

- update Cargo.toml dependencies

## [0.1.9](https://github.com/redbadger/crux/compare/crux_kv-v0.1.8...crux_kv-v0.1.9) - 2024-02-02

### Fixed

- fix doc test deps

### Other

- Make rust fmt happy
- Export crux_macros from crux_core and change docs
- More human readable change logs

## [0.1.8](https://github.com/redbadger/crux/compare/crux_kv-v0.1.7...crux_kv-v0.1.8) - 2024-01-26

### Other

- Add async API

## [0.1.7](https://github.com/redbadger/crux/compare/crux_kv-v0.1.6...crux_kv-v0.1.7) - 2024-01-11

### Other

- update Cargo.toml dependencies

## [0.1.6](https://github.com/redbadger/crux/compare/crux_kv-v0.1.5...crux_kv-v0.1.6) - 2023-12-03

### Other

- updated the following local packages: crux_core

## [0.1.5](https://github.com/redbadger/crux/compare/crux_kv-v0.1.4...crux_kv-v0.1.5) - 2023-11-29

### Other

- update dependencies

## [0.1.4](https://github.com/redbadger/crux/compare/crux_kv-v0.1.3...crux_kv-v0.1.4) - 2023-10-25

### Other

- versions for compatibility with semver checks
- implement derive macro for Capability trait
