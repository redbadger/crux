# Building capabilities

We covered effects and commands in detail, and hinted throughout at capabilities: the developer-friendly APIs you actually use when writing core code. Time to look at them directly, both using them and building our own.

In practice, apps need a fairly limited number of capabilities (typically around seven, almost certainly fewer than ten). The weather app uses six: Render, KeyValue, Http, Location, Secret, and Time. Capabilities are reusable across apps, so if you build one that others would benefit from, the Crux team would like to hear about it.

## Using a capability

Capabilities don't return a `Command` directly; they return a command *builder*, which lets you chain behaviour before committing to a specific event. We saw the abstract shape in [Managed Effects](./effects.md): `Http::get(...).expect_json().build().then_send(Event::ReceivedResponse)`.

The weather app's current-weather fetch shows the same pattern in production code:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/http/weather/mod.rs:fetch}}
```

`Http::get(...)` starts a builder, `.expect_json::<T>()` pins down the response type, `.query(...)` adds URL parameters, `.build()` produces a `RequestBuilder`, and `.map(...)` translates the shell's `Result<Response, HttpError>` into the more convenient `Result<CurrentWeatherResponse, WeatherError>`. The caller finishes it off with `.then_send(SomeEvent)`: `fetch` returns a builder, not a command, so callers can hook it into their own event type.

Note that a 4xx or 5xx response arrives on the **`Err`** side of that `Result`, never as an `Ok(Response)` carrying an error status. See [Handling `crux_http` rejections](../guide/http-rejections.md) for how to read one and how to test it.

That's how a capability gets used. But where do these APIs come from? Let's build one.

## One output per operation

Before we write any code, one rule shapes everything that follows: **an operation type has exactly one output type, and exactly one operation kind.**

The output is what the shell answers with. The `Operation` trait ties it to the operation:

```rust,ignore
// Rust
pub trait Operation: Send + 'static {
    type Output: Send + Unpin + 'static;
}
```

Because each operation is its own type, the only answer the shell can give it is a value of its `Output`.

The kind says *how many times* the shell answers:

- **notify**: the shell is told, and never answers. `Output` is `()`.
- **request**: the shell answers exactly once, with the operation's `Output`.
- **stream**: the shell answers any number of times, each with an `Output`.

The kind is part of the type, so the compiler holds every call site to it: sending an operation with the wrong `Command` constructor fails to compile. [Type generation](../part-4/typegen.md) also passes it on to the shell, so each generated handler method has the right shape. A request returns its output, and a stream is handed a sink to send into.

You declare both with `#[derive(Operation)]`:

```rust,ignore
// Rust
use crux_core::macros::Operation;

/// Told to the shell, never answered.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

/// Answered exactly once.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(request, output = ValueResult)]
pub struct GetValue { pub key: String }

/// Answered a sequence of times.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(stream, output = Message)]
pub struct Subscribe;
```

The derive writes the `Operation` implementation, its `Output`, the kind, and the marker trait (`crux_core::operation::Notify`, `Request` or `Stream`) that goes with it, so they can't disagree.

````admonish note
Outputs must be types type generation can emit, which rules out
`std::result::Result`. Where an operation can fail, the convention is a
concrete two-variant enum, like `crux_kv`'s `ValueResult` (the output of the
`GetValue` example above):

```rust,ignore
// Rust
#[derive(Facet, Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub enum ValueResult {
    Ok(Value),
    Err(KeyValueError),
}
```

Capabilities usually keep a `From` impl to hand the developer-facing `Result`
alias back from their builders, so app code still works in `Result`.
````

```admonish note title="Coming from an earlier version?"
Capabilities used to be written as one enum of operations sharing one response
enum, with the kind chosen by whichever `Command` constructor was called. If
your app has capabilities like that, the
[Migrating to per-operation types](../guide/migrate-per-operation-types.md)
guide walks through the change. Apps upgrading from before `crux_http` 0.19
should also see [Migrating `crux_http` to native `http` types](../guide/migrate-crux-http.md).
```

## A simple custom capability: Location

`Render` ships in `crux_core`; `crux_http`, `crux_kv`, and `crux_time` are separate crates Crux publishes. Location services aren't. They work differently enough across platforms that a cross-platform crate would do more harm than good, and they're specific enough that we didn't want to maintain an official one either. So the weather app defines its own.

A capability is two things:

1. A protocol for talking to the shell: an operation type per operation, each with its output.
2. An ergonomic API for the core developer: usually a handful of command-builder functions.

Here's the whole protocol for Location:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/location/mod.rs}}
```

Location has two operations, and each has its own output: the shell answers `IsLocationEnabled` with a `bool` and `GetLocation` with an `Option<Location>`. Both declare `request`, so the shell answers each exactly once and the compiler rejects `Command::notify_shell(GetLocation)`.

`Location` itself needs no mention anywhere: type generation walks each output's shape and emits every type it finds, so `Option<Location>` is enough for the shells to get a `Location`.

The developer API is equally small:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/location/command.rs}}
```

Each function issues one operation and hands back its output.

Notice the generic signatures: both functions are generic over `Effect` and `Event`. The trait bound `Effect: From<Request<GetLocation>>` says the caller's `Effect` type must be able to wrap *that* request. Every `#[effect]`-generated enum implements it for each operation it lists. Bounding per operation rather than per capability is what lets an app carry `KvGet` and `KvSet` without ever hearing about `ListKeys`.

## A richer example: Secret

Location is about as minimal as a capability gets. Secret (storing, fetching, and deleting an API key) has three operations, and each one has different outcomes.

Fetching a secret either finds it or doesn't. Storing one either succeeds or fails, and so does deleting one. Each operation names only its own outcomes:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/secret/mod.rs}}
```

Secret has three operations, each with a two-variant output of its own. The shell's generated handler method for `FetchSecret`, for example, returns a `SecretFetchResponse`.

The operation names carry the capability's name for a reason that has nothing to do with Rust: type generation puts every operation of every capability into one namespace on the shell side, so a bare `Delete` here would clash with any other capability or app operation called `Delete`. Type generation rejects two shared types that generate the same name. An app-defined operation with a generic name is worth prefixing from the start.

The developer API is correspondingly plain:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/secret/command.rs}}
```

Each builder issues its request and returns the output unchanged.

Using these builders looks no different from the location ones: call `secret::command::fetch(API_KEY_NAME)` and finish with `.then_send(...)` to bind the eventual `SecretFetchResponse` to an event.

## Notifications and streams

Both capabilities above are requests. The notes example's pub/sub capability has one of each of the other two kinds, which makes it the best place to see them.

```rust
// Rust
{{#include ../../../examples/notes/shared/src/capabilities/pub_sub.rs:operations}}
```

`Publish` is a **notification**: the shell broadcasts the bytes and there is nothing to answer, so its `Output` is `()` and `#[operation(notify)]` takes no `output` argument at all. `Subscribe` is a **stream**: the shell resolves the request once per `Message` that arrives from a peer, for as long as the subscription lives.

The builders differ in the same way:

```rust
// Rust
{{#include ../../../examples/notes/shared/src/capabilities/pub_sub.rs:builders}}
```

`Command::stream_from_shell` produces a `StreamBuilder`, whose `.then_send` fires an event per item rather than once; `Command::notify_shell` produces a `NotificationBuilder`, which has no output to send anywhere. Try them the other way round and the compiler stops you:

```text
error[E0080]: evaluation panicked: this operation does not declare
OperationKind::Request; send it with notify_shell or stream_from_shell instead
```

```admonish note title="Where that error appears"
The check is a `const` assertion evaluated after monomorphisation, so it fires
on `cargo build`, `cargo test` or `cargo clippy --all-targets`, but *not* on
`cargo check` or in your editor. The
[operation-kind traits RFC](https://github.com/redbadger/crux/pull/629) proposes making
the kind an associated type of `Operation`, which would turn this into an
ordinary trait-bound error you see as you type.
```

On the shell side, [type generation](../part-4/typegen.md) uses the kinds to shape the handler interface: `publish` returns nothing, `subscribe` is handed a sink to send `Message`s into, and a request method returns its output.

## What capabilities provide

Putting it together, a capability gives you two things:

- **A protocol**: one operation type per operation, each declaring its single output and its kind, which together define the wire format between core and shell.
- **A developer API**: small command-builder functions that speak in convenient Rust types rather than the raw protocol.

In [ports-and-adapters](https://en.wikipedia.org/wiki/Hexagonal_architecture) vocabulary, capabilities are the ports; the shell-side code that actually carries out each operation is the adapter. The core expresses *what* it wants done; the shell decides *how* to do it. Keeping that separation tight is what makes the core portable.

Speaking of the shell, it's time to look at how these operations get carried out on each platform. That's the next chapter.
