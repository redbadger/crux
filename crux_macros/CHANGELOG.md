# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.10-rc.0](https://github.com/redbadger/crux/compare/crux_macros-v0.3.9...crux_macros-v0.3.10-rc.0) - 2024-05-20

### Other

- update docs, comments and book
- Remove remaining mentions of the app attribute for Effect macro
- Make WithContext generic over Event, not App, enabling App types to be generic

## [0.3.9](https://github.com/redbadger/crux/compare/crux_macros-v0.3.8...crux_macros-v0.3.9) - 2024-05-14

This is a minor maintenance release, with the most interesting change being a fix for the export derive macro to work with renamed `Effect` types. (see https://github.com/redbadger/crux/pull/221 for more info)

### Other

- deps
- Merge branch 'master' into relax-callback-bounds
- add test
- allow export derive macro to name effect
- deps
- update all deps and dioxus examples

## [0.3.8](https://github.com/redbadger/crux/compare/crux_macros-v0.3.7...crux_macros-v0.3.8) - 2024-03-24

### Other

- deps
- make http error a struct variant
- rust deps
- update counter example to new crux_http, WIP
- update examples to latest crux

## [0.3.7](https://github.com/redbadger/crux/compare/crux_macros-v0.3.6...crux_macros-v0.3.7) - 2024-02-02

### Changed

- Only works with `crux_core` 0.7.3 or later.
- You should now import the macros from `crux_core::macros` rather than from
  this crate directly. This should avoid compatibility issues between the core
  and the macros in the future.

## [0.3.6](https://github.com/redbadger/crux/compare/crux_macros-v0.3.5...crux_macros-v0.3.6) - 2024-01-26

### Other

- darling default
- unimplemented rather than todo
- docs for effect macro
- effect derive macro allows skipping variants (e.g. for Never operations)
- update Rust deps

## [0.3.5](https://github.com/redbadger/crux/compare/crux_macros-v0.3.4...crux_macros-v0.3.5) - 2024-01-11

### Other

- update deps for Rust, Web, iOS and Android

## [0.3.4](https://github.com/redbadger/crux/compare/crux_macros-v0.3.3...crux_macros-v0.3.4) - 2023-11-29

### Other

- root deps

## [0.3.3](https://github.com/redbadger/crux/compare/crux_macros-v0.3.2...crux_macros-v0.3.3) - 2023-10-25

### Other

- update deps
- update leptos examples to remove Scope
- deps + tweaks
- deps
- deps
- deps
- deps, http 0.4.1, time 0.1.4
- update deps, iOS and Android examples
