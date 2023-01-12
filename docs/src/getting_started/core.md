# Shared core and types

These are the steps to set up the two crates forming the shared core – the core itself, and the shared types crate which does type generation for the foreign languages.

> _SHARP EDGE WARNING_: Most of these steps are going to be automated in future tooling, and published as crates. For now the set up is effectively a copy & paste from one of the [example projects](https://github.com/redbadger/crux/tree/master/examples).

## Install the tools

This is an example of a `rust-toolchain.toml` file that should ensure the correct compile targets are installed automatically for you. Add it to the root of your repo:

<!--- includes fail when indented see https://github.com/rust-lang/mdBook/pull/1718 --->

```toml
{{#include ../../../rust-toolchain.toml}}
```

Install the `uniffi-bindgen` binary. The version you install should match the dependencies in your `Cargo.toml` files.

```sh
cargo install uniffi_bindgen
```

## Create the core crate

Create a new rust library, like this:

```sh
cargo new --lib shared
```

Edit `./Cargo.toml` to add the new library to the Cargo workspace. It should look something like this:

```toml
{{#include ../../../examples/hello_world/Cargo.toml}}
```

Edit `./shared/Cargo.toml` to look something like this:

```toml
{{#include ../../../examples/hello_world/shared/Cargo.toml}}
```

Note that the `crate-type`

- `"lib"` is the default rust library when linking into a rust binary, e.g. in the `web-yew`, or `cli`, variant
- `"staticlib"` is a static library (`libshared.a`) for including in the Swift iOS app variant
- `"cdylib"` is a c-abi dynamic library (`libshared.so`) for use with JNA when included in the Kotlin Android app variant

Create `./shared/src/shared.udl`, like this:

```txt
{{#include ../../../examples/counter/shared/src/shared.udl}}
```

Create `./shared/uniffi.toml`, like this:

```toml
{{#include ../../../examples/counter/shared/uniffi.toml}}
```

Include the scaffolding in `./shared/src/lib.rs`, like this:

```rust
{{#include ../../../examples/counter/shared/src/lib.rs}}
```

Create a basic app implementation in `./shared/src/app.rs`, like this:

```rust
{{#include ../../../examples/hello_world/shared/src/counter.rs:1:45}}
```

Make sure everything builds OK

```sh
cargo build
```

## Create the shared types crate

This crate serves as the container for type generation for the foreign languages.

1. Copy over the [shared_types](https://github.com/redbadger/crux/tree/master/examples/counter/shared_types) folder from the counter example.

1. Edit the `build.rs` file and make sure to only list types you need.

1. Make sure everything builds and foreign types get generated into the `generated` folder.

   ```sh
   cargo build -vv
   ```

You should now be ready to set up [iOS](ios.md), [Android](android.md), [web](web_react.md), or [WebAssembly](web_yew.md) specific builds.
