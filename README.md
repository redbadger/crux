# Rust Multi-platform Mobile (RMM) ;-)

This repo is a "hello world"-style demonstration of how a single shared library (written in Rust) can act as the single, central core for cross-platform apps deployed on iOS, Android, or the Web. 

Unlike React Native, the user interface layer is built natively, with modern declarative UI frameworks such as Swift UI, Jetpack Compose and React/Vue or a WASM based framework on the web. The UI layer is as thin as it can be, and all other work is done by the shared core.

# Architectural Overview

![Architecture](./architecture.png)

The fundamental architectural concept used here is the strict separation of pure computational tasks from tasks that cause side effects.
This concept has been borrowed from [Elm](https://guide.elm-lang.org/architecture/).

### Side-effect-free core

In the above diagram, the inner "Cross-platform Rust core" is compiled and linked to the shell on each platform
as a library (native on iOS, [Java Native Access](https://github.com/java-native-access/jna) and with WebAssembly in the web browsers).

As such, the core is completely isolated and secure against software supply-chain attacks, as it has
no access to any external APIs. All it can do is perform calculations and keep state.

The core defines the key component types of the application:

- `Model` describing the internal state of the application
- `Message` an `enum` describing the events which the core can handle
- `ViewModel` representing information that should be displayed to the user

The former two are tied together by the `update` function, familiar from Elm, Redux or other event sourcing architectures. It has this type signature:

```rust
fn update(message: Message, model: &mut Model) -> Vec<Command<Message>>
```

Its job is to process a message, update the model based on it and potentially request some side-effects using the `Command`s. (The `Command` type is generic over the `Message` because each command specifies the message to be dispatched when it's completed).

### Application Shell

The enclosing "Platform native shell" is written using the language appropriate for the platform, and acts as the runtime environment within which all the non-pure tasks are performed. From the perspective of the core, the shell is the platform on which the core runs.

## Communication Between the Application and Core

Following the Elm architecture, the interface with the core is message based, and the core is unable to perform anything but calculation. To preform any side-effects, such as HTTP calls or random number generation, the core has to request them from the shell.

The core has a concept of Capabilities - reusable interfaces for common side-effects with request/response
semantics.

This means the core interface is very simple:

- `message: Message -> Vec<Request>` - processes a user interaction event and potentially responds with capability requests
- `response: Response -> Vec<Request>` - handles the response from the capability and potentially follows up with further requests
- `view: () -> ViewModel` - provides the shell with the current data for displaying user interface

Updating the user interface is is considered one of the capabilities.

This design means the core is very easily testable without any mocking and stubbing by simply checking Input/Ouput behaviour of the three
functions.

 The Foreign Function Interface allowing the shell to call the above functions is provided by Mozilla's [UniFFI](https://mozilla.github.io/uniffi-rs/) or [wasm-pack](https://rustwasm.github.io/wasm-pack/) in the browser.

 In order to send more complex data than UniFFI currently supports, the messages are serialised, sent across the boundary and deserialised using [serde_generate](https://docs.rs/serde-generate/latest/serde_generate/) which provides type generation for the foreign (not Rust) languages.

### Message Types

Two types of message are exchanged between the application and the core.

- Messages of type `Message` are sent from the Shell to the Core in response to an event happening in the user interface. They start a possible sequence of message exchanges between the shell and the core. It is passed on unchanged.
- Messages of type `Request` are sent from the Core to the Shell in response to either a `Message` or a `Response`, to request side-effects be performed byt the Shell.
- Messages of type `Response` are sent from the Shell to the Core as means of passing in results of an earlier request (synchronous or asynchronous).

`Request` and `Response` contain the useful payload for the capability, along with a `uuid` used by the 
core to keep track of the continuations, i.e. what message should be dispatched when a `Response` has
been received. The exact mechanics are not important, but it is important for the request's uuid to be passed on to the corresponding response.

## Typical Message Exchange Cycle

A typical message exchange cycle is as follows:

1. User interaction occurs in the shell that raises an event
1. The application handles this event by constructing a message
1. The application calls the core's `message` function passing the `Message` instance as an argument
1. The core performs the required processing, updating both its inner state and the view model
1. The core returns one or more `Request` messages to the application

In the simplest case, the core will respond to a `Message` by returning the single request - render (produced by the `update` function returning a `Command::Render`).

This requests that the shell re-renders the user interface. The absence of other requests also means this cycle has terminated (the core has "settled"). 

In more complex cases however, the core may well return multiple commands; each of which instructs the application to perform a side-effect-inducing task such as:

* Make a network call, or
* Fetch the current date/time stamp, or
* Perform biometric authentication, or
* Obtain an image from the camera, or
* Whatever else you can think of...

Many of these side-effecting-generating tasks are asynchronous.
The application then packages these responses into further `Response`s that are then passed to the core for processing.

This exchange continues until the core returns a `Cmd::Render` signalling that no more side-effects are in flight.

## Run the Example Locally

Refer to the [local execution](./docs/local-execution.md) README

## How to Start Your Own New Project

Refer to the [new project](./docs/new-project.md) README
