# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/redbadger/crux/compare/crux_time-v0.5.1...crux_time-v0.6.0) - 2024-10-23

### Added

- adds a new `Clear` variant to to the `TimeRequest` `Operation` and augments `NotifyAt` and `NotifyAfter`
  with a `TimerId` to facilitate cancelling requests. This is a breaking change.

### Other

- tidy and docs update
- remove unused test event

## [0.5.1](https://github.com/redbadger/crux/compare/crux_time-v0.5.0...crux_time-v0.5.1) - 2024-10-21

- Fixes a problem building the crate for the `typegen` feature, see https://github.com/redbadger/crux/pull/277

## [0.5.0](https://github.com/redbadger/crux/compare/crux_time-v0.4.4...crux_time-v0.5.0) - 2024-08-13

### Other
- merge 0.8.1 to master

## [0.4.4](https://github.com/redbadger/crux/compare/crux_time-v0.4.3...crux_time-v0.4.4) - 2024-08-12

### Other
- updated the following local packages: crux_core

## [0.4.3](https://github.com/redbadger/crux/compare/crux_time-v0.4.2...crux_time-v0.4.3) - 2024-05-21

### Other

- Release crux_core v0.8.0

## [0.4.2](https://github.com/redbadger/crux/compare/crux_time-v0.4.1...crux_time-v0.4.2) - 2024-05-15

### Other

- remove unused deps

## [0.4.1](https://github.com/redbadger/crux/compare/crux_time-v0.4.0...crux_time-v0.4.1) - 2024-05-14

### Other

- deps
- Merge branch 'master' into relax-callback-bounds
- address comments
- relax vaious func traits from Fn to FnOnce

## [0.4.0](https://github.com/redbadger/crux/compare/crux_time-v0.3.1...crux_time-v0.4.0) - 2024-04-29

### Other

- some better names
- add duration from millis
- update doc comments
- chrono behind feature, more conversions
- add subscribe_instant and subscribe_duration to crux_time

## [0.3.1](https://github.com/redbadger/crux/compare/crux_time-v0.3.0...crux_time-v0.3.1) - 2024-03-24

### Fixed

- fix a link error in README

## [0.3.0](https://github.com/redbadger/crux/compare/crux_time-v0.2.0...crux_time-v0.3.0) - 2024-02-02

### Breaking change

- Output type is now `TimeResponse(String)` (again) instead of
  `TimeResponse(chrono::DateTime<Utc>)`, in order to avoid typegen problems
  (lack of support for generic types).

### Fixed

- fix doc test deps

### Other

- TimeResponse(String) and fix up cat_facts examples
- update `crux_time` to not use generic response type + overall deps
- Export crux_macros from crux_core and change docs
- More human readable change logs

## [0.2.0](https://github.com/redbadger/crux/compare/crux_time-v0.1.8...crux_time-v0.2.0) - 2024-01-26

### Breaking change

- Move to using `chrono::DateTime<Utc>` as representation
- The main method is now called `now` instead of `get` and has a different
  return type

### Other

- Add async API to crux_time

## [0.1.8](https://github.com/redbadger/crux/compare/crux_time-v0.1.7...crux_time-v0.1.8) - 2024-01-11

### Other

- update Cargo.toml dependencies

## [0.1.7](https://github.com/redbadger/crux/compare/crux_time-v0.1.6...crux_time-v0.1.7) - 2023-12-03

### Other

- updated the following local packages: crux_core

## [0.1.6](https://github.com/redbadger/crux/compare/crux_time-v0.1.5...crux_time-v0.1.6) - 2023-11-29

### Other

- update dependencies

## [0.1.5](https://github.com/redbadger/crux/compare/crux_time-v0.1.4...crux_time-v0.1.5) - 2023-10-25

### Other

- versions for compatibility with semver checks
