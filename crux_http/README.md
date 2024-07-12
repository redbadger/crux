# Crux HTTP capability

[![Crate version](https://img.shields.io/crates/v/crux_http.svg)](https://crates.io/crates/crux_http)
[![Docs](https://img.shields.io/badge/docs.rs-crux_http-green)](https://docs.rs/crux_http/)

This crate contains the `Http` capability, which can be used to ask the Shell to make an HTTP request.

For an example of how to use the capability, see the [integration test](./tests/with_shell.rs).

The code for this was largely copied from [`surf`](https://github.com/http-rs/surf) with some modifications made to fit into the crux paradigm.

## Getting Started

Add `crux_http` as a dependency in your app's `Cargo.toml`.
Add `crux_http`, with feature `typegen`, as a build dependency in your `share_types` crate's `Cargo.toml`.

## About Crux Capabilities

Crux capabilities teach Crux how to interact with the shell when performing side effects. They do the following:

1. define a `Request` struct to instruct the Shell how to perform the side effect on behalf of the Core
1. define a `Response` struct to hold the data returned by the Shell after the side effect has completed
1. declare one or more convenience methods for invoking the Shell's capability, each of which creates a `Command` (describing the effect and its continuation) that Crux can "execute"

> Note that because Swift has no namespacing, there is currently a requirement to ensure that `Request` and `Response` are unambiguously named (e.g. `HttpRequest` and `HttpResponse`).
