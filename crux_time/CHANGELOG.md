# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.0](https://github.com/redbadger/crux/compare/crux_time-v0.11.0...crux_time-v0.12.0) - 2025-04-09

### Other

- updated the following local packages: crux_core

## [0.11.0](https://github.com/redbadger/crux/compare/crux_time-v0.10.1...crux_time-v0.11.0) - 2025-04-09
Replaces yanked 0.10.1 as that broke typegen on older versions of crux_core.

## [0.10.1](https://github.com/redbadger/crux/compare/crux_time-v0.10.0...crux_time-v0.10.1) - 2025-04-08

Updated to use version 0.13 of [`crux_core`](https://crates.io/crates/crux_core).

## [0.10.0](https://github.com/redbadger/crux/compare/crux_time-v0.9.0...crux_time-v0.10.0) - 2025-03-21

Breaking changes:
  - Introduced `TimerOutcome` to distinguish between timer cancellation and completion

### Other

- Make time effects deterministic, and avoid emitting immediately cancelled ones
- Fix a bug in the command runtime
- Improved panic message
- TimerOutcome should be Clone
- Introduce TimerOutcome and notes example
- add reverse PartialEq for handles
- update tests and examples
- Fix clippys
- Do not leak TimerResponse outside of the capability when using commands
- test cancellation with app

## [0.9.0](https://github.com/redbadger/crux/compare/crux_time-v0.8.3...crux_time-v0.9.0) - 2025-03-17

**This is a breaking-change release**

This release changes the public API of crux_time.

Whilst the protocol over the bridge has not changed, the implementation has been updated
to use `std::time::SystemTime` instead of `crux_time::Instant`
and to use `std::time::Duration` instead of `crux_time::Duration`.

### Other

- add PartialEq and Eq to TimerHandle
- update rust dependencies
- panic on overflow for protocol types
- use `std::time::SystemTime` and `std::time::Duration` in crux_time API

## [0.8.3](https://github.com/redbadger/crux/compare/crux_time-v0.8.2...crux_time-v0.8.3) - 2025-02-18

### Other

- Updates internal tests for error handling when resolving requests

## [0.8.2](https://github.com/redbadger/crux/compare/crux_time-v0.8.1...crux_time-v0.8.2) - 2025-02-03

### Other

- Command interface for crux_time
- Rely on FusedFuture skipping the select branch

## [0.8.1](https://github.com/redbadger/crux/compare/crux_time-v0.8.0...crux_time-v0.8.1) - 2025-01-22

### Other

- update dependencies
- update examples

## [0.8.0](https://github.com/redbadger/crux/compare/crux_time-v0.7.0...crux_time-v0.8.0) - 2025-01-21

### Other

- Integrate Commands into the Core so apps can mix and match
- Update App trait to support Command, fix all tests

## [0.7.0](https://github.com/redbadger/crux/compare/crux_time-v0.6.0...crux_time-v0.7.0) - 2025-01-07

### Breaking change

The API has been improved, which is a breaking change. More details in these PR descriptions:

- [crux_time API improvement](https://github.com/redbadger/crux/pull/284)
- [Cancelling a timer also aborts the cancelled task](https://github.com/redbadger/crux/pull/292)


### Other

- :Now is struct-like
- improve crux_time API
- remove pin_project
- add some more comments
- cancelling a timer also aborts the cancelled task

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
