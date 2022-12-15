# Crux — Cross-platform app development in Rust

<p>
  <a href="https://crates.io/crates/crux_core"><img alt="Crate Info" src="https://img.shields.io/crates/v/crux_core.svg"/></a>
  <a href="https://docs.rs/crux_core/"><img alt="API Docs" src="https://img.shields.io/badge/docs.rs-crux_core-green"/></a>
</p>

Crux helps you share your app's business logic and behavior across mobile (iOS and Android) and web, as a single, reusable core built with Rust.

Unlike React Native, the user interface layer is built natively, with modern declarative UI frameworks such as Swift UI, Jetpack Compose and React/Vue or a WASM based framework on the web.

The UI layer is as thin as it can be, and all other work is done by the shared core. The interface with the core has static type checking across languages.

> Note, that Crux is experimental and currently under active development (probably not ready for use in production apps just yet). The master branch should always be working well though, and we will try to keep the examples and documentation up to date as we go. The API hasn't settled yet, so beware! :-)

# Architectural Overview

![Architecture](./crux_core/architecture.png)

The fundamental architectural concept is the strict separation of pure computational tasks from tasks that cause side effects.
This is similar to the way [Elm](https://guide.elm-lang.org/architecture/) works.

### Side-effect-free core

In the above diagram, the inner "Cross-platform Rust core" is compiled and linked to the shell on each platform as a library:

- On iOS as a native static library
- On Android as a dynamic library using [Java Native Access](https://github.com/java-native-access/jna)
- In a browser as a WebAssembly module

In fact, because WebAssembly (Wasm) is one of the compilation targets, the core _must_ remain side-effect free, due to the sandboxed nature of the Wasm runtime environment.

As such, the core is completely isolated and secure against software supply-chain attacks, as it has
no access to any external APIs.
All it can do is perform pure calculations and keep internal state.

Following the Elm architecture, the core defines the key component types within the application:

- `Model` — describes the internal state of the application
- `Event` — an `enum` describing the events which the core can handle
- `ViewModel` — represents information that should be displayed to the user

The former two are tied together by the `update` function, familiar from Elm, Redux or other event sourcing architectures, which currently has this type signature:

```rust
fn update(
    &self,
    event: MyEvent,
    model: &mut MyModel,
    capabilities: &Capabilities,
)
```

The job of the `update` function is to process an `Event`, update the model accordingly, and potentially request some side-effects using capabilities.

### Application Shell

The enclosing "Platform native shell" is written using the language appropriate for the platform, and acts as the runtime environment within which all the non-pure tasks are performed.
From the perspective of the core, the shell is the platform on which the core runs.

## Communication Between the Application Shell and the Core

Following the Elm architecture, the interface with the core is message based.
This means that the core is unable to perform anything other than pure calculations.
To perform any task that creates a side-effect (such as an HTTP call or random number generation), the core must request it from the shell.

The core has a concept of Capabilities — reusable interfaces for common side-effects with request/response semantics. There are now a few embryonic Capability crates (Http, KeyValue, Time, Platform, and the builtin Render) — and you can write your own if you need/want to.

![crux](./docs//src/crux.png)

This means the core interface is simple:

- `message: Event -> Vec<Request>` - processes a user interaction event and potentially responds with capability requests. This is the API for the _driving_ side in the above diagram.
- `response: (uuid, SomeResponse) -> Vec<Request>` - handles the response from the capability and potentially follows up with further requests. This is the API for the _driven_ side in the above diagram.
- `view: () -> ViewModel` - provides the shell with the current data for displaying user interface

Updating the user interface is considered a side-effect and is provided by the built-in `Render` capability.

This design means the core can be tested very easily, without any mocking and stubbing, by simply checking the Input/Output behaviour of the three functions.

### Foreign Function Interface

The Foreign Function Interface allowing the shell to call the above functions is provided by Mozilla's [UniFFI](https://mozilla.github.io/uniffi-rs/) on a mobile device, or in the browser, by [wasm-pack](https://rustwasm.github.io/wasm-pack/).

In order to both send more complex data than UniFFI currently supports, and enforce the message passing semantics, all messages are serialized, sent across the boundary, then deserialized using [serde_generate](https://docs.rs/serde-generate/latest/serde_generate/) which also provides type generation for the foreign (non-Rust) languages.

This means that changes to types in the core, especially the `Event` and `Request` types, propagate out into the shell implementations and cause type errors where appropriate (such as an exhaustive match on an enum check).

### Message Types

Three types of message are exchanged between the application and the core.

- Messages of type `Event` are sent from the Shell to the Core in response to an event happening in the user interface (the _driving_ side).
  They start a potential sequence of further message exchanges between the shell and the core.
  Messages are passed on unchanged.
- Messages of type `Request` are sent from the Core to the Shell to request the execution of some side-effect-inducing task.
  The Core responds with zero or more `Request` messages after receiving an `Event` message (the _driven_ side).
- Response messages are sent from the Shell to the Core carrying the result of an earlier request.

`Request` messages contain the inputs for the requested side-effect, along with a `uuid` used by the core to pair requests and their responses together.
The exact mechanics are not important, but it is important for the request's `uuid` to be passed on to the corresponding response.

## Typical Message Exchange Cycle

A typical message exchange cycle may look like this:

1. User interaction occurs in the Shell, which results in an event
1. The Shell handles this event by constructing an `Event`
1. The Shell calls the Core's `message` function passing the `Event` as an argument
1. The Core performs the required processing, updating both its inner state and the view model
1. The Core returns one or more `Request` messages to the Shell

In the simplest case, the Core will respond to an `Event` by returning the single `Request` - render.

This requests that the Shell re-renders the user interface.
When `Render` is the only response from the Core, the message cycle has completed and the Core has now "settled".

In more complex cases however, the Core may well return multiple `Request`s; each of which instructs the Shell to perform a side-effect-inducing task such as:

- Make a network call, or
- Fetch the current date/time stamp, or
- Perform biometric authentication, or
- Obtain an image from the camera, or
- Whatever else you can think of...

Many of these side-effecting-inducing tasks are asynchronous.
The Shell is responsible for passing responses back to the core, which may respond with further requests.

This exchange continues until the core stops requesting further side-effects (typically the last side-effect requested would be `Render`).

## Run the Cat Facts Example Locally

Refer to [examples/cat_facts](./examples/cat_facts/README.md) README

## How to Start Your Own New Project

Refer to the [new project](./docs/new-project.md) README
