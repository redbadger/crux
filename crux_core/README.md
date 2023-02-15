# Crux — Cross-platform app development in Rust

<p>
  <a href="https://crates.io/crates/crux_core"><img alt="Crate Info" src="https://img.shields.io/crates/v/crux_core.svg"/></a>
  <a href="https://docs.rs/crux_core/"><img alt="API Docs" src="https://img.shields.io/badge/docs.rs-crux_core-green"/></a>
</p>

Crux helps you share your app's business logic and behavior across mobile (iOS and Android) and web, as a single, reusable core built with Rust.

Unlike [React Native](https://reactnative.dev/), but like [Kotlin Multi-platform Mobile](https://kotlinlang.org/lp/mobile/), the user interface layer is built natively, with modern declarative UI frameworks such as [SwiftUI](https://developer.apple.com/xcode/swiftui/), [Jetpack Compose](https://developer.android.com/jetpack/compose) and [React](https://reactjs.org/)/[Vue](https://vuejs.org/) or a Wasm based framework (like [Yew](https://yew.rs/)) on the web.

The UI layer is as thin as it can be, and all other work is done by the shared core. The interface with the core has static type checking across languages.

> Note, that Crux is experimental and currently under active development (probably not ready for use in production apps just yet). However, the master branch should always be working well, and we will try to keep the examples and documentation up to date as we go. We _do_ think that the API has now settled, so have a play! :-)

# Getting Started

- Read the [book](https://redbadger.github.io/crux)
- Follow the [readme in the project's repository on Github](https://github.com/redbadger/crux).
- Read the [API documentation](https://docs.rs/crux_core/latest/crux_core/)

# Architectural Overview

![Logical architecture](./architecture.svg)

The fundamental architectural concept is the strict separation of pure computational tasks from tasks that cause side effects.
This is similar to the way [Elm](https://guide.elm-lang.org/architecture/) works.

### Side-effect-free core

In the above diagram, the inner "Core" is compiled and linked to the outer "Shell" on each platform as a library:

- On iOS as a native static library
- On Android as a dynamic library using [Java Native Access](https://github.com/java-native-access/jna)
- In a browser as a WebAssembly module

In fact, because WebAssembly (Wasm) is one of the compilation targets, the core _must_ remain side-effect free, due to the sandboxed nature of the Wasm runtime environment.

As such, the core is completely isolated and secure against software supply-chain attacks, as it has no access to any external APIs.
All it can do is perform pure calculations and keep internal state.

Following the Elm architecture, the core defines the key component types within the application:

- `Event` — an `enum` describing the events which the core can handle
- `Model` — describes the internal state of the application
- `ViewModel` — represents information that should be displayed to the user

The former two are tied together by the `update` function, familiar from Elm, Redux or other event sourcing architectures, which currently has this type signature:

```rust
fn update(
    &self,
    event: Event,
    model: &mut Model,
    capabilities: &Capabilities,
)
```

The job of the `update` function is to process an `Event`, update the model accordingly, and potentially request some side-effects using capabilities.

### Application Shell

The enclosing "Platform native shell" is written using the language appropriate for the platform, and acts as the runtime environment within which all the non-pure tasks are performed.
From the perspective of the core, the shell is the platform on which the core runs.
