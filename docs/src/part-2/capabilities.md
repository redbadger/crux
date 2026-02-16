# Building capabilities

The final piece of the puzzle we should look at in our exploration
of the Weather app before we move to the Shell is Capabilities.

We looked at effects a fair bit and explored the `Command`s and `CommandBuilder`s,
but in practice, it's quite rare that you'd interact with those directly from
your app.

Typically, you'll be working with effects using Capabilities.

## Included capabilities

The weather app uses two out of the three capabilities provided with Crux: HTTP client (`crux_http`),
Key-Value store (`crux_kv`) and time capability (`crux_time`).

These are the most common things we think people will want to use in their apps. There are more,
and we will probably build those over time as well.

Let's look at the use of `crux_http` quickly, as it's the most extensive of the three. The Weather
app makes a pretty typical move and centralises the weather API use in a client:

```rust
{{#include ../../../examples/weather/shared/src/weather/client.rs:client}}
```

The main method there is `fetch`, which creates a GET request expecting a json response which
deserialises into a specific type, and provides a URL query to specify the search. At the end
of that chained call is a `.map` unpicking the response and turning it into a more convenient
`Result` type for the app code.

The interesting thing here is that the `fetch` method returns a `RequestBuilder`. In a way, this
makes it a half-way step to a custom capability, but it also just means the `fetch` call is
convenient to use from both normal and `async` context.

This is one of the things capabilities do - they map the lower-level protocols into a more
convenient API for the app developer.

Let's look at the other thing they do.

## Custom capabilities

The Weather app has one specialty - it works with location services. This is an example of a
capability which we'd probably struggle to find a cross-platform crate for.

The capability defines two things:

1. The protocol for communicating to the Shell
2. The APIs used by the programmer of the Core

Here is Weather app's Location capability in full:

```rust
{{#include ../../../examples/weather/shared/src/location/capability.rs}}
```

There are two interesting types: `LocationOperation` and `LocationResult` - they are the
request and response pair for the capability. The capability tells Crux that `LocationResult`
is the expected output for the `LocationOperation` with the trait implementation at the very
bottom. It marks the `LocationOperation` as an `Operation` as defined by Crux and associates
the output type.

That's number 1 done - protocol defined. This is what the Shell will need to understand and
return back in order to implement the location capability.

The rest of the code are the two APIs used by the Core developer - `is_location_enabled` and `get_location`.
Their type signatures are fairly complex, so lets pick them apart.

First, they are both generic over Effect and Event. This isn't strictly necessary for local
capabilities, but it makes the capability reusable for any `Effect` and `Event`, not just the
ones from the Weather app.

The other interesting thing is the trait bound `Effect: From<Request<LocationOperation>>`,
which says that the Effect type needs to be able to convert from a location Request, or
in other words - we need to be able to wrap a `Request<LocationOperation>` into the app's
Effect type. All Effect types generated with the `#[effect]` macro already do this.

Other than that, the APIs just create command builds and return them. Those types are also
somewhat gnarly, but it's mostly the `impl Future<Output = [value]>`, that's interesting.
Notice that the Output types are not `LocationResult`, they are the specific convenient
type the Core developer wants.

And that's all Capabilities do - they provide a convenient API for creating `CommandBuilder`s,
and converting between convenient Rust types and an FFI "wire protocol" used to communicate
with the Shell.

In the ports and adapters architecture, Capabilities are the ports, and the shell-side
implementations are the adapters.

In fact, let's go build one in the next chapter.
