# Preparing to add the Shell

So far, we've built a basic app in relatively basic Rust. If we now
want to expose it to a Shell written in a different language, we'll
have to set up the necessary plumbing, starting with the foreign function interface.

## The core FFI bindings

From the work so far, you may have noticed the app has a pretty limited API,
basically the `update` and `view` methods. There's one more for resolving
effects (called `resolve`), but that really is it. We need to make those three methods available
to the Shell, but once that's done, we don't have to touch it again.

Let's briefly talk about what we want from this interface. Ideally, in our language of choice we would:

- have a native equivalent of the `update`, `view` and `resolve` function
- have an equivalent for our `Event`, `Effect` and `ViewModel` types
- not have to worry about what black magic is happening behind the scenes to make that work

Crux provides code generation support for all of the above.

```admonish note
It isn't in any way actual black magic. What happens is Crux exposes FFI calls taking and returning
the values serialized with `bincode` (by default), and generated "foreign" (Swift, Kotlin, ...)
types handling the foreign side of the serialization.

Yes, this introduces some extra work to the FFI, but generally, for each user interaction we
make a relatively small number of round-trips (almost certainly less than ten), and our benchmarks say
we can make thousands of them per second. The real throughput _is_ dependent on how much data gets serialized,
but it only becomes a problem with _really_ large messages, and advanced workarounds exists. You
most likely don't need to worry about it, at least not for now.
```

## Preparing the core

We will prepare the core for both kinds of supported shells - native ones and webassembly ones.

To help with the native setup, Crux uses Mozilla's [Uniffi](https://mozilla.github.io/uniffi-rs/)
to generate the bindings. For webassembly, it uses [wasm-bingen](https://wasm-bindgen.github.io/wasm-bindgen/).

First, lets update our `Cargo.toml`:

```toml,ignore
# shared/Cargo.toml
{{#include ../../../examples/simple_counter/shared/Cargo.toml}}
```

A lot has changed! The key things we added are:

1. a `bin` target called `codegen`, which is how we're going to run all the code generation
2. feature flags to optionally enable `uniffi` and `wasm_bindgen`, and grouped those under `codegen` alongside some dependencies which are optional depending on that feature flag being enabled
3. dependencies we need for the code generation

And since we've declared the `codegen` target, we need to add the code for it.

```rust
// shared/src/bin/codegen.rs
{{#include ../../../examples/weather/shared/src/bin/codegen.rs}}
```

This is essentially boilerplate for a CLI we can use to run the binding generation and type generation.
But it's also a place where you can customize how they work if you have some more advanced needs.

It uses the `facet` based type generation from `crux_core` to scan the `App` for types which will cross
the FFI boundary, collect them and then, depending on what language should be generated builds the code
for it and places it into a specified `output_dir` directory.

We will call this CLI from the shell projects shortly.

### Codegen, typegen, bindgen, which is it?

You'll here these terms thrown around here and there in the docs, so it's worth clarifying what we mean

**bindgen** – "bindings generation" – provides APIs in the foreign language to call the core's Rust FFI APIs.
For most platforms we use UniFFI, except for WebAssembly, where we use `wasm_bindgen`

**typegen** – "type generation" – The core's FFI interface operates on bytes, but both Rust and the languages we're targeting are generally strongly typed. To facilitate the serialisation / deserialisation, we generate type definition reflecting the Rust types from the core in the foreign language (Swift, Kotlin, TypeScript, ...), which all serialise consistently.

**codegen** – you guessed it, "code generation" – is the two things above combined.

## Bindings code

No we need to add the Rust side of the bindings into our code. Update your `lib.rs` to look like this:

```rust,noplayground
// shared/src/lib.rs
{{#include ../../../examples/simple_counter/shared/src/lib.rs}}
```

This code uses our feature flags to conditionally initialise the UniFFI bindings and check the version
in use.

More importantly, it introduced a new `ffi.rs` module. Let's look at it closer:

```rust
// shared/src/ffi.rs
{{#include ../../../examples/simple_counter/shared/src/ffi.rs}}
```

Broad strokes: we define a type for core with FFI, which holds a `Bridge` wrapping our `Counter`, and
provide implementations of the three API methods taking and returning byte buffers.

The translation between rust types and the byte buffers is the job of the bridge (it also holds the
effect requests inside the core under an id, which can be sent out to the Shell and used to resolve the
effect, but more on that later).

Notice the Shell is in charge of creating the instance of this type, so in theory your Shell can have
several instances of the app if it wants to.

There are many attribute macros annotating the FFI type for `uniffi` and `wasm_bindgen`, which generate
the actual code making them available as FFIs. We recommend the respective documentation if you're
interested in the detail of how this works. The notable part is that both libraries have a level of support for
various basic and structured data types which we don't use, and instead we serialize the data with Serde,
and generate types with `facet_generate` to make the support consistent.

It's not essential for you to understand the detail of the above code now. You won't need to change it, unless you're
doing something fairly advanced, by which time you'll understand it.

## Platform native part

Okay, with that plumbing, the Core part of adding a shell is complete. It's not a one liner, but you will only
set this up once, and most likely won't touch it again, but having the ability, should you need to, is important.

Now we can proceed to the actual shell for your platform of choice:

- [iOS with Swift and SwiftUI](./shell/ios/index.md)
- [Android with Kotlin and Jetpack Compose](./shell/android/index.md)
- [Web with TypeScript, React and Next.js](./shell/web/react.md)
- [Rust in WebAssembly with Leptos](./shell/web/leptos.md)
