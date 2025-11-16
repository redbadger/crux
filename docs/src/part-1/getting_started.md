# Getting started

These are the steps to set up the two crates forming the shared core – the core
itself, and the shared types crate which does type generation for the foreign
languages.

```admonish warning title="Sharp edge"
We're hoping to automate some of these steps in future tooling. For now the set up includes some copy & paste from one of the [example projects](https://github.com/redbadger/crux/tree/master/examples).
```

## Install the tools

This is an example of a
[`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
file, which you can add at the root of your repo. It should ensure that the
correct rust channel and compile targets are installed automatically for you
when you use any rust tooling within the repo.

<!--- includes fail when indented see https://github.com/rust-lang/mdBook/pull/1718 --->

```toml
{{#include ../../../rust-toolchain.toml}}
```

## Create the core crate

### The shared library

The first library to create is the one that will be shared across all platforms,
containing the _behavior_ of the app. You can call it whatever you like, but we
have chosen the name `shared` here. You can create the shared rust library, like
this:

```sh
cargo new --lib shared
```

### The workspace and library manifests

We'll be adding a bunch of other folders into the monorepo, so we are choosing
to use Cargo Workspaces. Edit the workspace `/Cargo.toml` file, at the monorepo
root, to add the new library to our workspace. It should look something like
this:

```toml
# /Cargo.toml
[workspace]
members = ["shared"]
resolver = "1"

[workspace.package]
authors = ["Red Badger Consulting Limited"]
edition = "2021"
repository = "https://github.com/redbadger/crux/"
license = "Apache-2.0"
keywords = ["crux", "crux_core", "cross-platform-ui", "ffi", "wasm"]
rust-version = "1.88"

[workspace.dependencies]
anyhow = "1.0.95"
crux_core = "0.12.0"
serde = "1.0.217"
```

The library's manifest, at `/shared/Cargo.toml`, should look something like the
following, but there are a few things to note:

- the `crate-type`
    - `lib` is the default rust library when linking into a rust binary, e.g. in
      the `web-yew`, or `cli`, variant
    - `staticlib` is a static library (`libshared.a`) for including in the Swift
      iOS app variant
    - `cdylib` is a C-ABI dynamic library (`libshared.so`) for use with JNA when
      included in the Kotlin Android app variant
- we need to declare a feature called `typegen` that depends on the feature with
  the same name in the `crux_core` crate. This is used by this crate's sister
  library (often called `shared_types`) that will generate types for use across
  the FFI boundary (see the section below on generating shared types).
- the uniffi dependencies and `uniffi-bindgen` target should make sense after
  you read the next section

```toml
# /shared/Cargo.toml
{{#include ../../../examples/simple_counter/shared/Cargo.toml}}
```

### Building your app

TODO: Just file
