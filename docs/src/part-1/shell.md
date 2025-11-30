# Preparing to add the Shell

So far, we've built a basic app in relatively basic Rust. If we now
want to expose it to a Shell written in a different language, we'll
have to set up the necessary plumbing.

## The core FFI bindings

From the work so far, you may have noticed the app has a pretty limited API,
basically the `update` and `view` methods. There's one more for resolving
effects, but that really is it. We need to make those three methods available
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
[package]
name = "shared"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

[lib]
crate-type = ["lib", "staticlib", "cdylib"]
name = "shared"

[[bin]]
name = "codegen"
required-features = ["codegen"]

[features]
uniffi = ["dep:uniffi"]
wasm_bindgen = ["dep:wasm-bindgen", "dep:getrandom"]
codegen = [
    "crux_core/cli",
    "dep:clap",
    "dep:log",
    "dep:pretty_env_logger",
    "uniffi",
]
facet_typegen = ["crux_core/facet_typegen"]

[dependencies]
clap = { version = "4.5.53", optional = true, features = ["derive"] }
crux_core.workspace = true
facet = { version = "=0.30", features = ["chrono", "time"] }
log = { version = "0.4.28", optional = true }
pretty_env_logger = { version = "0.5.0", optional = true }
serde = { workspace = true, features = ["derive"] }
uniffi = { version = "=0.29.4", optional = true }
wasm-bindgen = { version = "0.2.105", optional = true }
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
But it's also a place where you can do customize how they work if you have some more advanced needs.
It uses the `facet` based type generation from `crux_core` to scan the `App` for types which will cross
the FFI boundary, collect them and then, depending on what language should be generated builds the code
for it and places it into a specified `output_dir` directory.

We will call this CLI from the shell projects shortly.

## Bindings code

No we need to add the Rust side of the bindings into our code. Update your `lib.rs` to look like this:

```rust,noplayground
mod app;
#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
mod ffi;

pub use app::Counter;

#[cfg(any(feature = "wasm_bindgen", feature = "uniffi"))]
pub use ffi::CoreFFI;

#[cfg(feature = "uniffi")]
const _: () = assert!(
    uniffi::check_compatible_version("0.29.4"),
    "please use uniffi v0.29.4"
);
#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
```

This code uses our feature flags to pick which kind of FFI to use.

Let's look at the FFI module we also introduced there:

```rust
// shared/src/ffi.rs
use crux_core::{
    Core,
    bridge::{Bridge, EffectId},
};

use crate::app::Counter;

/// The main interface used by the shell
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
pub struct CoreFFI {
    core: Bridge<Counter>,
}

impl Default for CoreFFI {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "uniffi", uniffi::export)]
#[cfg_attr(feature = "wasm_bindgen", wasm_bindgen::prelude::wasm_bindgen)]
impl CoreFFI {
    #[cfg_attr(feature = "uniffi", uniffi::constructor)]
    #[cfg_attr(
        feature = "wasm_bindgen",
        wasm_bindgen::prelude::wasm_bindgen(constructor)
    )]
    #[must_use]
    pub fn new() -> Self {
        Self {
            core: Bridge::new(Core::new()),
        }
    }

    /// Send an event to the app and return the effects.
    /// # Panics
    /// If the event cannot be deserialized.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn update(&self, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.update(data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Resolve an effect and return the effects.
    /// # Panics
    /// If the `data` cannot be deserialized into an effect or the `effect_id` is invalid.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn resolve(&self, id: u32, data: &[u8]) -> Vec<u8> {
        let mut effects = Vec::new();
        match self.core.resolve(EffectId(id), data, &mut effects) {
            Ok(()) => effects,
            Err(e) => panic!("{e}"),
        }
    }

    /// Get the current `ViewModel`.
    /// # Panics
    /// If the view cannot be serialized.
    /// In production you should handle the error properly.
    #[must_use]
    pub fn view(&self) -> Vec<u8> {
        let mut view_model = Vec::new();
        match self.core.view(&mut view_model) {
            Ok(()) => view_model,
            Err(e) => panic!("{e}"),
        }
    }
}
```

Broad strokes: we define a type for core with FFI, which holds a `Bridge` wrapping our `Counter`, and
provide implementations of the key methods taking and returning byte buffers. The translation between
rust types and the byte buffers is the job of the bridge (it also holds the effect requests inside the
core under an id, which can be sent out to the Shell and used to resolve the effect, but more on that later).

Notice the Shell is in charge of creating the instance of this type, so in theory your Shell can have
several instances of the app if it wants to.

There are many attribute macros annotating the FFI type for `uniffi` and `wasm_bindgen`, which generate
the actual code making them available as FFIs. We recommend the respective documentation if you're interested  
in the detail of how this works. The notable part is that both libraries have a level of support for
various basic and structured data types which we don't use, and instead we serialize the data with Serde,
and generate types with `facet_generate` to make the support consistent.

## Platform native part

Okay, with that plumbing, the Core part of adding a shell is complete. It's not a one liner, but you will only
set this up once, and most likely won't touch it again, but having the ability to should you need to is important.

Now we can proceed to the actual shell for your platform of choice:

- [iOS with Swift and SwiftUI](./shell/ios/index.md)
- [Android with Kotlin and Jetpack Compose](./shell/android/index.md)
- [Web with TypeScript, React and Next.js](./shell/web/react.md)
- [Rust in WebAssembly with Leptos](./shell/web/leptos.md)
