# Setting up

This section is the fast path. It builds the counter app the way we recommend
writing a Crux app: a shared core in Rust, and shells that use the
`Core` class Crux generates for them, so all a shell writes is its UI and an
`EffectHandler` for the app's effects.

The finished code is in
[`examples/counter`](https://github.com/redbadger/crux/tree/master/examples/counter),
and the chapters that follow walk through the parts of it that matter. They
explain just enough to get you going, and link to the rest of the book where
the details live:

- [Part I](../part-1/basic_app.md) builds the same counter with a
  hand-written shell, to show how the shell and the core talk to each other.
- [Part II](../part-2/weather_app.md) builds a real app, with HTTP, storage,
  timers and location, and goes into managed effects and capabilities in
  depth.

We generally recommend building Crux apps from the inside out, starting with
the Core. But first, we need to make sure we have all the necessary tools.

## Install the tools

This is an example of a
[`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file)
file, which you can add at the root of your repo. It should ensure that the
correct rust channel and compile targets are installed automatically for you
when you use any rust tooling within the repo.

You may not need all the targets if you're not planning to build a fully cross platform app.

```toml
# TOML: /rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = [
    "aarch64-apple-darwin",
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-linux-android",
    "wasm32-unknown-unknown",
    "x86_64-apple-ios",
]
profile = "minimal"
```

For testing, we also recommend to install [`cargo-nextest`](https://nexte.st/), the test runner we'll be using
in the examples.

```sh
cargo install cargo-nextest --locked
```

The shells link the core through [BoltFFI](https://www.boltffi.dev/), and the
examples run their build steps with the [Just](https://just.systems/) task
runner, so install those too:

```sh
cargo install boltffi_cli --version '=0.30.1' --locked
cargo install just
```

Each shell also needs its own platform's tools: Xcode, Android Studio, Node.js
with `pnpm`, or .NET. In the example, running `just doctor` in a shell's
directory checks you have what that shell needs.

## Create the core crate

We need a crate to hold our application's core, but since one of our shell options later will
be rust based, we'll set up a cargo workspace to have some isolation between the core and the
other Rust based modules

### The workspace and library manifests

First, create a workspace and start with a `/Cargo.toml` file, at the monorepo
root, to add the new library to our workspace.

It should look something like this:

```toml
# TOML: /Cargo.toml
[workspace]
resolver = "3"
members = ["shared"]

[workspace.package]
edition = "2024"
rust-version = "1.90"

[workspace.dependencies]
anyhow = "1.0.104"
crux_core = "0.21"
serde = "1.0.229"
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
# TOML: /shared/Cargo.toml
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[dependencies]
crux_core.workspace = true
serde = { workspace = true, features = ["derive"] }
```

### The basic files

The only missing part now is your `src/lib.rs` file. This will eventually
contain a fair bit of configuration for the shell interface, so we tend to
recommend reserving it to this job and creating a `src/app.rs` module
for your app code.

For now, the `lib.rs` file looks as follows:

```rust,noplayground
// Rust: src/lib.rs
mod app;

pub use app::*;
```

and `app.rs` can be empty, but let's put our app's main type in it,
call it `Counter`:

```rust,noplayground
// Rust: src/app.rs

#[derive(Default)]
pub struct Counter;
```

Running

```sh
cargo build
```

should build your Core. Let's make it [do something now](./core.md).
