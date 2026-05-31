# Preparing to add the Shell

So far, we've built a basic app in relatively basic Rust. If we now
want to expose it to a Shell written in a different language, we'll
have to set up the necessary plumbing, starting with the foreign function interface.

## The core FFI bindings

From the work so far, you may have noticed the app has a pretty limited API,
basically the `update` and `view` methods. There's one more for resolving
effects (called `resolve`), but that really is it. We need to make those three methods available
to the Shell, but once that's done, we don't have to touch it again.

Let's briefly talk about what we want from this interface. Ideally, in our shell language we would:

- have native equivalents of the `update`, `view` and `resolve` functions
- have an equivalent for our `Event`, `Effect` and `ViewModel` types
- not have to worry about what black magic is happening behind the scenes to make that work

Crux provides code generation support for all of the above.

```admonish note
It isn't in any way actual black magic. What happens is Crux exposes FFI calls taking and returning
the values serialized with `bincode` (by default), and generated "foreign" (Swift, Kotlin, TypeScript, ...)
types handling the foreign side of the serialization.

Yes, this introduces some extra work to the FFI, but generally, for each user interaction we
make a relatively small number of round-trips (almost certainly less than ten), and our benchmarks say
we can make thousands of them per second. The real throughput _is_ dependent on how much data gets serialized,
but it only becomes a problem with _really_ large messages, and advanced workarounds exist. You
most likely don't need to worry about it, at least not for now.
```

## Preparing the core

We will prepare the core for native, WebAssembly, and C# shells.

Crux uses [BoltFFI](https://www.boltffi.dev/) for the small byte-oriented FFI
surface. Crux's type generation remains separate: Facet-generated Swift,
Kotlin, TypeScript, and C# types handle the app's serialized
`Event`/`Effect`/`ViewModel` data.

First, let's update our `Cargo.toml`:

```toml,ignore
# shared/Cargo.toml
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["cdylib", "lib", "staticlib"]

[[bin]]
name = "codegen"
required-features = ["codegen"]

[features]
facet_typegen = ["crux_core/facet_typegen"]
codegen = [
    "dep:anyhow",
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "facet_typegen"
]

[dependencies]
boltffi = { git = "https://github.com/boltffi/boltffi", tag = "v0.25.2" }
facet = "=0.44"
crux_core.workspace = true
serde = { workspace = true, features = ["derive"] }

# optional dependencies
anyhow = { workspace = true, optional = true }
clap = { version = "4.6.1", optional = true, features = ["derive"] }
log = { version = "0.4.29", optional = true }
pretty_env_logger = { version = "0.5.0", optional = true }
```

A lot has changed! The key things we added are:

1. a `bin` target called `codegen`, which is how we're going to run all the code generation
2. a `boltffi` dependency for the binding surface
3. dependencies we need for the code generation

And since we've declared the `codegen` target, we need to add the code for it.

```rust,noplayground
// shared/src/bin/codegen.rs
{{#include ../../../examples/counter/shared/src/bin/codegen.rs}}
```

This is essentially boilerplate for a CLI we can use to run type generation.
But it's also a place where you can customize type generation if you have more
advanced needs.

It uses the Facet-based type generation from `crux_core` to scan the `App` for
types that cross the FFI boundary, collect them, and write the requested
language output into the specified `output_dir` directory.

We will call this CLI from the shell projects shortly.

### Codegen, typegen, bindgen, which is it?

You'll hear these terms thrown around here and there in the docs, so let's
clarify what we mean:

**bindgen** – "bindings generation" – provides APIs in the foreign language to call the core's Rust FFI APIs.
Crux uses BoltFFI for native, web, and C# bindings.

**typegen** – "type generation" – The core's FFI interface operates on bytes,
but both Rust and the languages we're targeting are generally strongly typed.
To support serialization and deserialization, we generate foreign type
definitions that reflect the Rust types from the core (Swift, Kotlin,
TypeScript, ...), all with consistent serialization behavior.

**codegen** – you guessed it, "code generation" – combines the two.

### The BoltFFI config file

One more file is worth calling out before we move on:
[`shared/boltffi.toml`](https://www.boltffi.dev/docs/configuration).

```toml,ignore
# shared/boltffi.toml
{{#include ../../../examples/counter/shared/boltffi.toml}}
```

BoltFFI reads this file when the shell recipes package Apple, Android, and wasm
targets, or generate C# bindings. The `[package]` table identifies the Rust
crate being packaged. The `[targets.*]` tables describe each shell package:
where to write generated artifacts, what Swift module or Kotlin package to use,
where to put the wasm/npm output, and how to configure the C# bindings.

These paths are relative to `shared/`, because the BoltFFI commands run from
that directory. If you rename the crate or move a shell, update this file and
the matching shell project together.

## Updating our `app.rs`

There's a few things we need to do to our `app.rs` module to support typegen.
The first thing we need to do is update the annotation of the `Effect` type to
tell our `effect` attribute macro to use the Facet-based typegen path.

```rust,noplayground
#[derive(Debug)]
#[effect(facet_typegen)] // previously #[effect]
pub enum Effect {
    Render(RenderOperation),
}
```

We also need to annotate the other types that cross the FFI boundary with the
`Facet` derive macro. We are using Facet v0.44 (with `crux_core` v0.17), and so
we also need to specify a layout for enums, e.g. `repr(C)` or `repr(u8)`.

```rust,noplayground
use facet::Facet;

// derive Facet and specify layout
#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum Event {
    Increment,
    Decrement,
    Reset,
}

// derive Facet
#[derive(Facet, Serialize, Deserialize, Clone, Default)]
pub struct ViewModel {
    pub count: String,
}
```

## Bindings code

Now we need to add the Rust side of the bindings into our code. Update your `lib.rs` to look like this:

```rust,noplayground
// shared/src/lib.rs
{{#include ../../../examples/counter/shared/src/lib.rs}}
```

This code exposes the `ffi.rs` module, where BoltFFI sees the byte-oriented
`CoreFFI` class. Let's look at it closer:

```rust,noplayground
// shared/src/ffi.rs
{{#include ../../../examples/counter/shared/src/ffi.rs}}
```

Broad strokes: `CoreFFI` holds a `Bridge` wrapping `Counter` and exposes the
core API as methods taking and returning byte buffers.

The translation between Rust types and byte buffers is the bridge's job. It
also holds the effect requests inside the core under an id, which can be sent
out to the Shell and used to resolve the effect, but more on that later.

Notice the Shell is in charge of creating the instance of this type, so in theory your Shell can have
several instances of the app if it wants to.

The `#[boltffi::export]` attribute marks the Rust class and methods that should
be made available to shell languages. BoltFFI can support richer shapes, but Crux
keeps this layer deliberately tiny: app data is serialized with Serde and
`bincode`, and `facet_generate` creates matching host-language types so behavior stays
consistent across platforms.

It's not essential for you to understand the detail of the above code now. You
won't need to change it unless you're doing something fairly advanced, by which
time you'll understand it.

## Platform native part

Okay, with that plumbing in place, the Core side of adding a shell is complete.
It's not a one-liner, but you will only set it up once, and most likely won't
touch it again.

Now we can proceed to the actual shell for your platform of choice:

- [iOS with Swift and SwiftUI](./shell/apple/index.md)
- [Android with Kotlin and Jetpack Compose](./shell/android/index.md)
- [Web with TypeScript, React and Next.js](./shell/web/react.md)
- [Rust in WebAssembly with Leptos](./shell/web/leptos.md)
