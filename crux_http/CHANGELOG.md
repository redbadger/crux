# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.1](https://github.com/redbadger/crux/compare/crux_http-v0.5.0...crux_http-v0.5.1) - 2024-02-02

### Fixed
- fix doc test deps
- fix clippy lints
- fix macro to trace Effect
- fix features on http-types

### Other
- update `crux_time` to not use generic response type + overall deps
- Merge branch 'master' into byo-serialization
- Make rust fmt happy
- Export crux_macros from crux_core and change docs
- remove http_types default features from crux_http
- More human readable change logs
- release
- Capabilities don't need to be clone
- Clone less, to lead by example
- Prefer IntoFuture as the main RequestBuilder async API
- Doc comment
- Remove irrelevant comment
- Add async API to crux_http
- update Rust deps
- release
- update deps for Rust, Web, iOS and Android
- release
- release
- root deps
- release
- versions for compatibility with semver checks
- update deps
- deps + tweaks
- deps
- deps
- deps
- capability doc tests
- deps
- deps, http 0.4.1, time 0.1.4
- :ok(), clone() to Time cap, +tweaks
- crux_core v0.4.0
- added json() builder methods and fixed up examples
- add builders for HttpRequest and HttpResponse
- add response headers to http capability protocol
- update deps, iOS and Android examples
- deps
- implement derive macro for Capability trait
- into_effects(), effects() and effects_mut()
- Update.effects as Iterator
- update deps
- v0.6.0
- Merge branch 'master' into bincode
- http_v0.3.1
- v0.5.0
- remove unneeded feature in crux_hhtp
- docs and doctests
- split out export derive macro
- http typegen test WIP
- deps
- version bumps
- Fix crux_http tests for the new API, amend HttpRequests in examples
- update http with shell tests
- Change Request from a tuple struct to a normal struct
- Rename Step to Request, change crux_core module structure
- Fix crux_http tests, extend testing support to work with steps
- Change Step to be easier to consume in Rust shells
- crux_http module update: added HTTPRequest body to protocol.rs
- deps
- Rename rustdoc::missing_doc_code_examples lint
- deps
- Add support for HTTP request headers
- prep for 0.3 release
- update deps
- some typos
- rename core fn names
- deps
- add MSRV
- deps
- Don't default to HTTP 1.1 in test responses
- Attribute surf in the crux_http README
- Remove crux_http::Http::send_
- Remove crux_http::protocol::HttpMethod
- Don't re-export the protocol types from crux_http.
- Write a quick unit test of crux_http::Client
- Remove crux_http::Config docstrings.
- Update crux_http::Request docstrings
- Update ResponseAsync doctests
- Fix the Response doctests
- Update the examples to use new HTTP capability
- Fix some more doctests
- Reinstate redirect middleware
- Fix docstrings for crux_http::RequestBuilder
- More doc tests
- Start on fixing the documentation
- Fix some CI problems
- Clean up a ton of warnings
- Merge remote-tracking branch 'origin/master' into steal-an-http-api
- workspace deps
- Add & use Capability::Operation associated type
- move macros dep in caps to dev
- simplify tests by naming better
- ability to specify Event type name
- update uniffi and other deps
- Merge branch 'master' into counter-example
- Merge branch 'master' into counter-example
- Merge branch 'master' into testing-effects-with-async
- example test for http cap using tester + clippy fixes
- First pass of capability test helping funtimes
- Merge remote-tracking branch 'origin/master' into graeme-just-going-all-in-on-async
- remove Default bounds from App impl in tests
- use macro in tests + capability fields can be private
- and update tests
- stronger typing inside http cap
- extract kv cap and write test
- extract http capability into own crate

## [0.5.0](https://github.com/redbadger/crux/compare/crux_http-v0.4.6...crux_http-v0.5.0) - 2024-01-30

### Fixed
- fix doc test deps

### Other
- remove http_types default features from crux_http
- More human readable change logs

## [0.4.6](https://github.com/redbadger/crux/compare/crux_http-v0.4.5...crux_http-v0.4.6) - 2024-01-26

### Fixed
- fix clippy lints

### Other
- Add async API support

## [0.4.5](https://github.com/redbadger/crux/compare/crux_http-v0.4.4...crux_http-v0.4.5) - 2024-01-11

### Other
- update deps for Rust, Web, iOS and Android

## [0.4.4](https://github.com/redbadger/crux/compare/crux_http-v0.4.3...crux_http-v0.4.4) - 2023-12-03

### Other
- updated the following local packages: crux_core, crux_core

## [0.4.3](https://github.com/redbadger/crux/compare/crux_http-v0.4.2...crux_http-v0.4.3) - 2023-11-29

### Other
- root deps

## [0.4.2](https://github.com/redbadger/crux/compare/crux_http-v0.4.1...crux_http-v0.4.2) - 2023-10-25

### Other
- versions for compatibility with semver checks
- update deps
- deps + tweaks
- deps
- deps
- deps
- capability doc tests
- deps
