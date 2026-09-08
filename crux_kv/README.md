# Crux Key-Value Store capability

This crate contains the `KeyValue` capability, which can be used to ask the
Shell to read from, and write to, a key-value store.

Currently it provides an interface for getting, setting, and deleting keys,
checking if keys exists in the store, and listing keys that start with a prefix.

## Getting Started

Add `crux_kv` as a dependency in your app's `Cargo.toml`.

### Type generation

The types this capability sends over the bridge derive `Facet`, so Crux's
facet-based type generation picks them up: enable the crate's `facet_typegen`
feature alongside `crux_core/facet_typegen` and follow
[Type generation](https://redbadger.github.io/crux/part-4/typegen.html).

## About Crux Capabilities

Crux capabilities teach Crux how to interact with the shell when performing side
effects. They do the following:

1. define a `Request` struct to instruct the Shell how to perform the side
   effect on behalf of the Core
1. define a `Response` struct to hold the data returned by the Shell after the
   side effect has completed
1. declare one or more convenience methods for invoking the Shell's capability,
   each of which creates a `Command` (describing the effect and its
   continuation) that Crux can "execute"

> Note that because Swift has no namespacing, there is currently a requirement
> to ensure that `Request` and `Response` are unambiguously named (e.g.
> `HttpRequest` and `HttpResponse`).
