# Getting started

We generally recommend building Crux apps from inside out, starting with the Core.

This part will first take you through setting up the tools and building the Core,
and writing tests to make sure everything works as expected. Finally, once we're
confident we have a working core, we'll set up the necessary bindings for the shell
and build the UI for your chosen platform.

But first, we need to make sure we have all the necessary tools

## Install the tools

This is an example of a
[`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
file, which you can add at the root of your repo. It should ensure that the
correct rust channel and compile targets are installed automatically for you
when you use any rust tooling within the repo.

You may not need all the targets if you're not planning to build a fully cross platform app.

<!--- includes fail when indented see https://github.com/rust-lang/mdBook/pull/1718 --->

```toml
{{#include ../../../rust-toolchain.toml}}
```

For testing, we also recommend to install [`cargo-nextest`](https://nexte.st/), the test runner we'll be using
in the examples.

```sh
cargo install cargo-nextest
```

## Create the core crate

We need a crate to hold our application's core, but since one of our shell options later will
be rust based, we'll set up a cargo workspace to have some isolation between the core and the
other Rust based modules

### The workspace and library manifests

First, create a workspace and start with a `/Cargo.toml` file, at the monorepo
root, to add the new library to our workspace.

It should look something like this:

```toml
# /Cargo.toml
[workspace]
resolver = "3"
members = ["shared"]

[workspace.package]
edition = "2024"
rust-version = "1.88"

[workspace.dependencies]
anyhow = "1.0.100"
crux_core = "0.17.0"
serde = "1.0.228"
```

### The shared library

The first library to create is the one that will be shared across all platforms,
containing the _behavior_ of the app. You can call it whatever you like, but we
have chosen the name `shared` here. You can create the shared rust library, like
this:

```sh
cargo new --lib shared
```

The library's manifest, at `/shared/Cargo.toml`, should look something like the
following,

```toml
# /shared/Cargo.toml
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "lib", "staticlib"]
name = "shared"

[dependencies]
crux_core.workspace = true
serde = { workspace = true, features = ["derive"] }
```

Note the `crate-type` in the `[lib]` section. This is in preparation for linking with the
shells:

- `lib` is the default rust library when linking into a rust binary
- `staticlib` is a static library (`libshared.a`) for use with iOS apps
- `cdylib` is a C-ABI dynamic library (`libshared.so`) for use with JNA in an Android app

### The basic files

The only missing part now is your `src/lib.rs` file. This will eventually
contain a fair bit of configuration for the shell interface, so we tend to
recommend reserving it to this job and creating a a `src/app.rs` module
for your app code.

For now, the `lib.rs` file looks as follows:

```rust
// src/lib.rs
pub mod app;
```

and `app.rs` can be empty, but let's put our app's main type in it,
call it `Counter`:

```rust
// src/app.rs

#[derive(Default)]
pub struct Counter;
```

Running

```sh
cargo build
```

should build your Core. Let's make it [do something now](./basic_app.md).
