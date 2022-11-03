# Rust Multi-platform Mobile (RMM) ;-)

This repo is a "hello world"-style demonstration of how a single shared library (written in Rust) can act as the single, central core for both mobile apps deployed on either iOS or Android devices, or the Web.

# Architectural Overview

![Architecture](./architecture.png)

The fundamental architectural concept used here is the strict separation of pure computational tasks from tasks that cause side effects.
This concept has been borrowed from [Elm](https://guide.elm-lang.org/architecture/).

### Pure WebAssembly Core

In the above diagram, the inner "Cross-platform Rust core" is compiled to WebAssembly and therefore, due to the sand-boxed nature of a `.wasm` module, is only capable of performing pure computation.

### Non-Pure Application

The enclosing "Platform native shell" is written using whatever language is appropriate for the platform in question, and acts as the runtime environment within which all the non-pure tasks are performed.

Rendering the user interface is treated as a side effect handled by the application layer

## Communication Between the Application and Core

In order to have the same Rust core beneath both a Kotlin and a Swift UI layer, the binding and foreign function interface (FFI) calls have been generated using [`UniFFI`](https://github.com/mozilla/uniffi-rs)

For the Web, these bindings have been generated using [`yew`](https://yew.rs/).

The core also exposes a view model through its `view` function, which the application uses to determine which values should be rendered on the screen using the native UI toolkit.

The key benefits of building the core in this way are these:

- The core is side-effect free and does not make use of any system APIs.
This means it can be compiled to WebAssembly.
- The core can be tested without the need for mocking or stubbing.
All that is needed is to check that for a given inbound message, the core responds with the correct set of outbound messages and the correct view model.
- Thanks to UniFFI, all data types used in message passing are shared across the FFI boundary.
Then, when the code is updated (E.G. with new variants on the `Msg` type), type checking in the application layer (Swift and Kotlin) will cause the build to fail until the new messages are correctly handled.
This keeps everything in sync!

Message exchange is always initiated by the application &mdash; typically in response to some event in the user interface.
When such an event goes off, the application sends a message to the core that then performs the relevant (pure) business logic.
The core then responds by returning one or more messages.

It is important to understand that in order for the results of the core's computation to be visible in the "outside world", the core must send at least one message back to the application.

### Message Types

Two types of message are exchanged between the application and the core.

* Messages of type `Msg` are sent from the Application to the Core in response to either:

   1. An event happening in the user interface
   1. The result of processing a list of commands received from the core
   1. The response to an earlier asynchronous request (E.G. an HTTP request) has been received and must be sent to the core for processing

* Messages of type `Cmd` are sent from the Core to the Application

   Upon receipt of a `Msg`, the core performs the necessary pure computation and returns a list of one or more `Cmd` messages.
These commands then instruct the application what to do next.

## Typical Message Exchange Cycle

A typical message exchange cycle is as follows:

1. User interaction occurs in the application layer that raises an event
1. The application handles this event by constructing a message
1. The application calls the core's `update` function passing the `Msg` as an argument
1. The core performs the required processing, updating both its inner state and the view model
1. The core returns one or more `Cmd` messages to the application

In the simplest case, the core will respond to a `Msg` by returning the single command `Cmd::Render`.
This informs the application of two things:

1. The user interface needs to be re-rendered
1. This particular message exchange cycle has come to an end

In more complex cases however, the core may well return multiple commands; each of which instructs the application to perform a side-effect-inducing task such as:

* Make a network call, or
* Fetch the current date/time stamp, or
* Perform biometric authentication, or
* Obtain an image from the camera, or
* Whatever else you can think of...

Many of these side-effecting-generating tasks are asynchronous.
The application then packages these responses into further `Msg`s that are then passed to the core for processing.

This exchange continues until the core returns a `Cmd::Render` signalling that no more side-effects are in flight.

## Run the Example Locally

Refer to the [local execution](./docs/local-execution.md) README

## How to Start Your Own New Project

Refer to the [new project](./docs/new-project.md) README
