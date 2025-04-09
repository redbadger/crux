# Shared core and types

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
rust-version = "1.66"

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

### FFI bindings

Crux uses Mozilla's [Uniffi](https://mozilla.github.io/uniffi-rs/) to generate
the FFI bindings for iOS and Android.

#### Generating the `uniffi-bindgen` CLI tool

Since version `0.23.0` of Uniffi, we need to also generate the
binary that generates these bindings. This avoids the possibility of getting a
version mismatch between a separately installed binary and the crate's Uniffi
version. You can read more about it
[here](https://mozilla.github.io/uniffi-rs/tutorial/foreign_language_bindings.html).

Generating the binary is simple, we just add the following to our crate, in a
file called `/shared/src/bin/uniffi-bindgen.rs`.

```rust,ignore
{{#include ../../../examples/simple_counter/shared/src/bin/uniffi-bindgen.rs}}
```

And then we can build it with cargo.

```sh
cargo run -p shared --bin uniffi-bindgen

# or

cargo build
./target/debug/uniffi-bindgen
```

The `uniffi-bindgen` executable will be used during the build in XCode and in
Android Studio (see the following pages).

#### The interface definitions

We will need an interface definition file for the FFI bindings. Uniffi has its
own file format (similar to WebIDL) that has a `.udl` extension. You can create
one at `/shared/src/shared.udl`, like this:

```txt
{{#include ../../../examples/simple_counter/shared/src/shared.udl}}
```

There are also a few additional parameters to tell Uniffi how to create bindings
for Kotlin and Swift. They live in the file `/shared/uniffi.toml`, like this
(feel free to adjust accordingly):

```toml
# /shared/uniffi.toml
{{#include ../../../examples/simple_counter/shared/uniffi.toml}}
```

Finally, we need a `build.rs` file in the root of the crate
(`/shared/build.rs`), to generate the bindings:

```rust,no_run,noplayground
// /shared/build.rs
{{#include ../../../examples/simple_counter/shared/build.rs}}
```

### Scaffolding

Soon we will have macros and/or code-gen to help with this, but for now, we need
some scaffolding in `/shared/src/lib.rs`. You'll notice that we are re-exporting
the `Request` type and the capabilities we want to use in our native Shells, as
well as our public types from the shared library.

```rust,no_run,noplayground
// /shared/src/lib.rs
{{#include ../../../examples/simple_counter/shared/src/lib.rs}}
```

### The app

Now we are in a position to create a basic app in `/shared/src/app.rs`. This is
from the
[simple Counter example](https://github.com/redbadger/crux/blob/master/examples/simple_counter/shared/src/counter.rs)
(which also has tests, although we're not showing them here).

```rust,no_run,noplayground
// /shared/src/app.rs
{{#include ../../../examples/simple_counter/shared/src/app.rs:app}}
```

```admonish note title="Note the #[effect] macro"
The [`#[effect]`](https://docs.rs/crux_macros/latest/crux_macros/macro.effect.html) macro can be used to annotate an enum to represent our effects. The enum has a variant for each effect, which carries the [`Operation`](https://docs.rs/crux_core/latest/crux_core/capability/trait.Operation.html) type.

The real effect type generated by the macro is a little more complicated, with some plumbing to support the foreign function interface into Swift, Kotlin and other languages. You can read more about the effect system in the [Managed Effects](../guide/effects.md) chapter of the guide.
```

```admonish warning title="The Capabilities associated type"
The `Capabilities` associated type in the code above is an artifact of a migration of the effect API from
previous versions of Crux. You can use the unit type `()` and everything will work fine. We will
eventually remove this type and the last argument to the `update` function.

If you've got an existing app or you're simply curious about what this looked like before, you can read about it
at the end of the [Managed Effects](../guide/effects.md) chapter of the guide.
```

Make sure everything builds OK

```sh
cargo build
```

## Create the shared types crate

This crate serves as the container for type generation for the foreign
languages.

Work is being done to remove the need for this crate, but for now, it is needed
in order to drive the generation of the types that cross the FFI boundary.

- Copy over the
  [shared_types](https://github.com/redbadger/crux/tree/master/examples/simple_counter/shared_types)
  folder from the simple_counter example.

- Add the shared types crate to `workspace.members` in the `/Cargo.toml` file at the
  monorepo root.

- Edit the `build.rs` file and make sure that your app type is registered. In
  our example, the app type is `Counter`, so make sure you include this
  statement in your `build.rs`

```rust,ignore
 gen.register_app::<Counter>()?;
```

The `build.rs` file should now look like this:

```rust,no_run,noplayground
{{#include ../../../examples/simple_counter/shared_types/build.rs}}
```

If you are using the latest versions of the
`crux_http` (>= `v0.10.0`), `crux_kv` (>= `v0.5.0`) or `crux_time` (>= `v0.5.0`)
capabilities, you will need to add a build dependency to the capability crate,
with the `typegen` feature enabled — so your `Cargo.toml` file may end up looking something like this
(from the [`cat_facts`](https://github.com/redbadger/crux/tree/master/examples/cat_facts) example):

```toml
{{#include ../../../examples/cat_facts/shared_types/Cargo.toml}}
```

````admonish tip
Due to a current limitation with the reflection library,
you may need to manually register nested enum types in your `build.rs` file.
(see <https://github.com/zefchain/serde-reflection/tree/main/serde-reflection#supported-features>)


*Note, you don't have to do this for the latest versions of the
`crux_http` (>= `v0.10.0`), `crux_kv` (>= `v0.5.0`) or `crux_time` (>= `v0.5.0`)
capabilities, which now do this registration for you — although you will need to add
a build dependency to the capability crate, with the `typegen` feature enabled.*

If you _do_ end up needing to register a type manually (you should get a helpful error to tell you this),
you can use the `register_type` method (e.g. `gen.register_type::<TextCursor>()?;`) as
shown in this
[`build.rs`](https://github.com/redbadger/crux/blob/master/examples/notes/shared_types/build.rs)
file from the `shared_types` crate of the
[notes example](https://github.com/redbadger/crux/tree/master/examples/notes):

```rust,ignore
{{#include ../../../examples/notes/shared_types/build.rs}}
```
````

### Building your app

Make sure everything builds and foreign types get generated into the
`generated` folder.

(If you're generating TypeScript, you may need `pnpm` to be installed and in your `$PATH`.)

```sh
cargo build
```

````admonish tip
If you have a `Capabilities` struct (i.e. you are not using the new [`#[effect]`](https://docs.rs/crux_macros/latest/crux_macros/macro.effect.html) macro), and are having problems building, make sure your `Capabilities` struct also implements the `Export` trait.
There is a derive macro that can do this for you, e.g.:

```rust,ignore
#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    render: Render<Event>,
    http: Http<Event>,
}
```

The `Export` and `Effect` derive macros can be configured with the `effect` attribute if you need to specify a different name for the Effect type e.g.:

```rust,ignore
#[cfg_attr(feature = "typegen", derive(Export))]
#[derive(Effect)]
#[effect(name = "MyEffect")]
pub struct Capabilities {
    render: Render<Event>,
    pub_sub: PubSub<Event>,
}
```

Additionally, if you are using a Capability that does not need to be exported to the foreign language, you can use the `#[effect(skip)]` attribute to skip exporting it, e.g.:

```rust,ignore
#[cfg_attr(feature = "typegen", derive(Export))]
#[derive(Effect)]
pub struct Capabilities {
    render: Render<Event>,
    #[effect(skip)]
    compose: Compose<Event>,
}
```
````


```admonish success
You should now be ready to set up [iOS](iOS/index.md), [Android](Android/index.md), or [web](web/index.md) specific builds.
```
