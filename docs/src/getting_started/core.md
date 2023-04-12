# Shared core and types

These are the steps to set up the two crates forming the shared core – the core itself, and the shared types crate which does type generation for the foreign languages.

```admonish warning title="Sharp edge"
Most of these steps are going to be automated in future tooling, and published as crates. For now the set up is effectively a copy & paste from one of the [example projects](https://github.com/redbadger/crux/tree/master/examples).
```

## Install the tools

This is an example of a [`rust-toolchain.toml`](https://rust-lang.github.io/rustup/overrides.html#the-toolchain-file) file, which you can add at the root of your repo. It should ensure that the correct rust channel and compile targets are installed automatically for you when you use any rust tooling within the repo.

<!--- includes fail when indented see https://github.com/rust-lang/mdBook/pull/1718 --->

```toml
{{#include ../../../rust-toolchain.toml}}
```

## Create the core crate

### The shared library

The first library to create is the one that will be shared across all platforms, containing the _behavior_ of the app. You can call it whatever you like, but we have chosen the name `shared` here.
You can create the shared rust library, like this:

```sh
cargo new --lib shared
```

### The workspace and library manifests

We'll be adding a bunch of other folders into the monorepo, so we are choosing to use Cargo Workspaces. Edit the workspace `/Cargo.toml` file, at the monorepo root, to add the new library to our workspace. It should look something like this (the `package` and `dependencies` fields are just examples):

```toml
{{#include ../../../examples/hello_world/Cargo.toml}}
```

The library's manifest, at `/shared/Cargo.toml`, should look something like the following, but there are a few things to note:

- the `crate-type`
  - `lib` is the default rust library when linking into a rust binary, e.g. in the `web-yew`, or `cli`, variant
  - `staticlib` is a static library (`libshared.a`) for including in the Swift iOS app variant
  - `cdylib` is a C-ABI dynamic library (`libshared.so`) for use with JNA when included in the Kotlin Android app variant
- the `path` fields on the crux dependencies are for the [examples in the Crux repo](https://github.com/redbadger/crux/tree/master/examples) and so you will probably not need them
- the uniffi dependencies and `uniffi-bindgen` target should make sense after you read the next section

```toml
{{#include ../../../examples/hello_world/shared/Cargo.toml}}
```

### FFI bindings

Crux uses Mozilla's [Uniffi](https://mozilla.github.io/uniffi-rs/) to generate the FFI bindings for iOS and Android.

#### Generating the `uniffi-bindgen` CLI tool

Since Mozilla released version `0.23.0` of Uniffi, we need to also generate the binary that generates these bindings. This avoids the possibility of getting a version mismatch between a separately installed binary and the crate's Uniffi version. You can read more about it [here](https://mozilla.github.io/uniffi-rs/tutorial/foreign_language_bindings.html).

Generating the binary is simple, we just add the following to our crate, in a file called `/shared/src/bin/uniffi-bindgen.rs`.

```rust
{{#include ../../../examples/hello_world/shared/src/bin/uniffi-bindgen.rs}}
```

And then we can build it with cargo.

```sh
cargo run -p shared --bin uniffi-bindgen

# or

cargo build
./target/debug/uniffi-bindgen
```

The `uniffi-bindgen` executable will be used during the build in XCode and in Android Studio (see the following pages).

#### The interface definitions

We will need an interface definition file for the FFI bindings. Uniffi has its own file format (similar to WebIDL) that has a `.udl` extension. You can create one here `/shared/src/shared.udl`, like this:

```txt
{{#include ../../../examples/counter/shared/src/shared.udl}}
```

There are also a few additional parameters to tell Uniffi how to create bindings for Kotlin and Swift. They live in the file `/shared/uniffi.toml`, like this (feel free to adjust accordingly):

```toml
{{#include ../../../examples/counter/shared/uniffi.toml}}
```

### Scaffolding

Soon we will have macros and/or code-gen to help with this, but for now, we need some scaffolding in `/shared/src/lib.rs`. You'll notice that we are re-exporting the `Request` type and the capabilities we want to use in our native Shells, as well as our public types from the shared library.

```rust,noplayground
{{#include ../../../examples/counter/shared/src/lib.rs}}
```

### The app

Now we are in a position to create a basic app in `/shared/src/app.rs`. This is from the [simple Counter example](https://github.com/redbadger/crux/blob/master/examples/hello_world/shared/src/counter.rs) (which also has tests, although we're not showing them here):

```rust,noplayground
{{#include ../../../examples/hello_world/shared/src/counter.rs:1:52}}
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

```admonish success
You should now be ready to set up [iOS](ios.md), [Android](android.md), [web](web_react.md), or [WebAssembly](web_yew.md) specific builds.
```
