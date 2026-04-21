# Building capabilities

We covered effects and commands in detail, and hinted throughout at capabilities — the developer-friendly APIs you actually use when writing core code. Time to look at them directly, both using them and building our own.

In practice, apps need a fairly limited number of capabilities — typically around seven, almost certainly fewer than ten. The weather app uses six: Render, KeyValue, Http, Location, Secret, and Time. Capabilities are reusable across apps — if you build one that others would benefit from, the Crux team would like to hear about it.

## Using a capability

Capabilities don't return a `Command` directly — they return a command *builder*, which lets you chain behaviour before committing to a specific event. We saw the abstract shape in chapter 5: `Http::get(...).expect_json().build().then_send(Event::ReceivedResponse)`.

The weather app's current-weather fetch shows the same pattern in production code:

```rust
{{#include ../../../examples/weather/shared/src/effects/http/weather/mod.rs:fetch}}
```

`Http::get(...)` starts a builder, `.expect_json::<T>()` pins down the response type, `.query(...)` adds URL parameters, `.build()` produces a `RequestBuilder`, and `.map(...)` translates the shell's `Result<Response, HttpError>` into the more convenient `Result<CurrentWeatherResponse, WeatherError>`. The caller finishes it off with `.then_send(SomeEvent)` — `fetch` returns a builder, not a command, so callers can hook it into their own event type.

That's how a capability gets used. But where do these APIs come from? Let's build one.

## A simple custom capability: Location

`Render` ships in `crux_core`; `crux_http`, `crux_kv`, and `crux_time` are separate crates Crux publishes. Location services aren't — they work differently enough across platforms that a cross-platform crate would do more harm than good, and they're specific enough that we didn't want to maintain an official one either. So the weather app defines its own.

A capability is two things:

1. A protocol for talking to the shell — an operation type and a response type.
2. An ergonomic API for the core developer — usually a handful of command-builder functions.

Here's the whole protocol for Location:

```rust
{{#include ../../../examples/weather/shared/src/effects/location/mod.rs}}
```

Two operation variants (`IsLocationEnabled`, `GetLocation`), two result variants (`Enabled(bool)`, `Location(Option<Location>)`), and an `impl Operation for LocationOperation` pairing them. The `Operation` trait is Crux's way of saying "when you see this operation, expect this response type" — the macro-generated `Effect` type uses it so the core and shell agree on the wire format.

The developer API is equally small:

```rust
{{#include ../../../examples/weather/shared/src/effects/location/command.rs}}
```

Each function issues one operation and narrows the response. `is_location_enabled` returns `bool`; `get_location` returns `Option<Location>`. The shared `LocationResult` carries both variants, so each `.map(...)` pins the response to the one that operation expects and falls back to a safe default for the other — `false` for the enabled check, `None` for the location fetch. Secret, later in the chapter, uses `unreachable!()` for the same situation; both patterns have their place.

Notice the generic signatures: both functions are generic over `Effect` and `Event`. The trait bound `Effect: From<Request<LocationOperation>>` says the caller's `Effect` type must be able to wrap a location request — every `#[effect]`-generated enum implements this automatically, so the bound is always satisfied in practice. Being generic lets us drop this capability into any Crux app, not just this one.

## A richer example: Secret

Location is about as minimal as a capability gets. Secret — storing, fetching, and deleting an API key — has a bit more going on, and it shows a pattern worth calling out.

### Narrowing the shell's response

The shell's `SecretResponse` is a single enum with six variants: `Missing`, `Fetched`, `Stored`, `StoreError`, `Deleted`, `DeleteError`. Each operation has its own pair: `Fetch` produces `Missing` or `Fetched`, `Store` produces `Stored` or `StoreError`, and `Delete` produces `Deleted` or `DeleteError`. If a caller holds a `SecretResponse` directly, the type doesn't tell them which operation it's responding to — they'd have to handle variants that can't apply to their call.

The capability fixes this by defining three narrower response types — `SecretFetchResponse`, `SecretStoreResponse`, `SecretDeleteResponse` — and having each command builder return its own. The wide `SecretResponse` stays as the shell protocol; the core developer only ever sees the narrowed versions.

Here's the protocol:

```rust
{{#include ../../../examples/weather/shared/src/effects/secret/mod.rs}}
```

And the developer API:

```rust
{{#include ../../../examples/weather/shared/src/effects/secret/command.rs}}
```

Each builder issues a request, then `.map(...)` narrows the wide `SecretResponse` down to the operation-specific type. The `unreachable!()` calls document an invariant: because the shell only ever produces the "right" variants for a given operation, the other arms should never fire. If they do, there's a bug in the shell's handler that the panic surfaces rather than hides.

Using these builders looks no different to the location ones: call `secret::command::fetch(API_KEY_NAME)` and finish with `.then_send(...)` to bind the eventual `SecretFetchResponse` to an event.

## What capabilities provide

Putting it together, a capability gives you two things:

- **A protocol** — operation and response types marked with the `Operation` trait, which define the wire format between core and shell.
- **A developer API** — small command-builder functions that speak in convenient Rust types rather than the raw protocol.

In [ports-and-adapters](https://en.wikipedia.org/wiki/Hexagonal_architecture) vocabulary, capabilities are the ports; the shell-side code that actually carries out each operation is the adapter. The core expresses *what* it wants done; the shell decides *how* to do it. Keeping that separation tight is what makes the core portable.

Speaking of the shell — it's time to look at how these operations get carried out on each platform. That's the next chapter.
