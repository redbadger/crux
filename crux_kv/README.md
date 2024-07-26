# Crux Key-Value Store capability

This crate contains the `KeyValue` capability, which can be used to ask the
Shell to read from, and write to, a key-value store.

Currently it provides an interface for getting, setting, and deleting keys,
checking if keys exists in the store, and listing keys that start with a prefix.

## Getting Started

Add `crux_kv` as a dependency in your app's `Cargo.toml`.

### Typegen

This crate has a feature called `typegen` which supports generation of code
(e.g. in TypeScript, Swift, Kotlin etc.) for the types that the Capability
passes over the bridge.

Crux apps usually contain a `shared` crate for the behavioural "core" and a
`shared_types` crate that is responsible for generating the types that are
shared between the core and the shell.

The `shared` crate can re-export the capability with a `typegen` feature that
depends on the `typegen` feature of the Capability crate. This way, the shared
crate can ask the Capability to register its types for type generation.

e.g. in the `shared` crate's `Cargo.toml`:

```toml
[features]
typegen = ["crux_core/typegen", "crux_kv/typegen"]
```

and in the `shared_types` crate's `Cargo.toml`:

```toml
[build-dependencies]
crux_core = { workspace = true, features = ["typegen"] }
shared = { path = "../shared", features = ["typegen"] }
```

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
