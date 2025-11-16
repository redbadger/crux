# Adding the Shell

## The core FFI bindings

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

## The platform specific part

That is the core part of adding a shell complete, now we can proceed to the actual
shell for your platform of choice

- [iOS with Swift and SwiftUI](./shell/ios/index.md)
- [Android with Kotlin and Jetpack Compose](./shell/android/index.md)
- [Web with TypeScript, React and Next.js](./shell/web/react.md)
- [Rust in WebAssembly with Leptos](./shell/web/leptos.md)
