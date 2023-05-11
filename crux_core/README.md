# [Crux](https://red-badger.com/crux) &middot; [![GitHub license](https://img.shields.io/github/license/redbadger/crux?color=blue)](https://github.com/redbadger/crux/blob/master/LICENSE) [![Crate version](https://img.shields.io/crates/v/crux_core.svg)](https://crates.io/crates/crux_core) [![Docs](https://img.shields.io/badge/docs.rs-crux_core-green)](https://docs.rs/crux_core/) [![Build status](https://img.shields.io/github/actions/workflow/status/redbadger/crux/build.yaml)](https://github.com/redbadger/crux/actions)

<a href="https://red-badger.com/crux"><img src="./crux_logo.svg" height="100" /></a>

## Cross-platform app development in Rust

- **Shared Core for Behavior** - Crux helps you share your app's business logic and behavior across mobile (iOS/Android) and web — as a single reusable core built with Rust.
- **Thin Shell for UI** - Crux recognizes that the best experiences are built with modern declarative frameworks such as [SwiftUI](https://developer.apple.com/xcode/swiftui/), [Jetpack Compose](https://developer.android.com/jetpack/compose), [React](https://reactjs.org/)/[Vue](https://vuejs.org/), or a WebAssembly based framework (like [Yew](https://yew.rs/)) — however, it aims to keep this UI layer as thin as it can be, with all other work done by the shared core.
- **Type Generation** - the interface with the core has static type checking across languages — types and serialization code are generated for Swift, Kotlin and TypeScript. Rust shells can import the core directly.
- **Capabilities** - capabilities express the intent for side effects such as calling an API. Because all side effects (including UI) are performed by the shell, the core becomes trivial to test comprehensively — test suites run in milliseconds (not in minutes or hours).

## Getting Started

[Learn how to use Crux in your project](https://redbadger.github.io/crux).

Follow the [readme in the project's repository on Github](https://github.com/redbadger/crux).

Read the [API documentation](https://docs.rs/crux_core/latest/crux_core/)

[Watch the introductory talk](https://www.youtube.com/watch?v=cWCZms92-1g&t=5s) at the recent [Rust Nation 2023](https://www.rustnationuk.com/) conference in London.

You can also join the friendly conversation on our [Zulip channel](https://crux-community.zulipchat.com).

> Note, that Crux is experimental and currently under active development (probably not ready for use in production apps just yet). However, the master branch should always be working well, and we will try to keep the examples and documentation up to date as we go. We _do_ think that the API has now settled, so have a play! :-)

## Architectural Overview

![Logical architecture](./architecture.svg)

The fundamental architectural concept is the strict separation of pure computational tasks from tasks that cause side effects.
This is similar to the way [Elm](https://guide.elm-lang.org/architecture/) works.

### Side-effect-free core

In the above diagram, the inner "Core" is compiled and linked to the outer "Shell" on each platform as a library:

- On iOS as a native static library
- On Android as a dynamic library using [Java Native Access](https://github.com/java-native-access/jna)
- In a browser as a WebAssembly module

In fact, because WebAssembly (Wasm) is one of the compilation targets, the core _must_ remain side-effect free, due to the sandboxed nature of the Wasm runtime environment.

As such, the core is completely isolated and secure against software supply-chain attacks, as it has
no access to any external APIs.
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

The enclosing platform native "Shell" is written using the language appropriate for the platform, and acts as the runtime environment within which all the non-pure tasks are performed.
From the perspective of the core, the shell is the platform on which the core runs.
