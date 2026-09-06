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

Note that a 4xx or 5xx response arrives on the **`Err`** side of that `Result`, never as an `Ok(Response)` carrying an error status — see [Handling `crux_http` rejections](../guide/http-rejections.md) for how to read one and how to test it.

That's how a capability gets used. But where do these APIs come from? Let's build one.

## One output per operation

Before we write any code, one rule shapes everything that follows: **an operation type has exactly one output type, and exactly one request kind.**

The `Operation` trait has always said the first half:

```rust,ignore
pub trait Operation: Send + 'static {
    type Output: Send + Unpin + 'static;
}
```

The trouble is that it used to be conventional to implement it on a *coarse enum* — one operation type with five variants, one output type with five variants — and then the trait's promise stops being true in practice. Any response variant is a well-formed answer to any request variant, as far as the type system and the deserializer are concerned. So the capability has to check at runtime, and every capability author has to decide what to do when the check fails: panic, invent an error, or quietly ignore it. `crux_kv` and `crux_time` both used to panic.

So: one type per operation. Each carries its own output, and the wrong answer stops being expressible.

The second half — the request kind — is the same idea applied to *how many times* the shell answers:

- **notify** — the shell is told, and never answers. `Output` is `()`.
- **request** — the shell answers exactly once, with the operation's `Output`.
- **stream** — the shell answers any number of times, each with an `Output`.

That used to be decided by which `Command` constructor you called, so the same operation could be notified in one place and streamed in another. Declaring it on the type instead means the compiler can hold you to it, and — more usefully — it means [type generation](../part-4/typegen.md) can tell the *shell* how many times to resolve each effect, which it previously had to learn by reading Rust source.

You declare both with `#[derive(Operation)]`:

```rust,ignore
use crux_core::macros::Operation;

/// Told to the shell, never answered.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

/// Answered exactly once.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(request, output = ValueResult)]
pub struct Get { pub key: String }

/// Answered a sequence of times.
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(stream, output = Message)]
pub struct Subscribe;
```

The derive writes the `Operation` implementation, its `Output`, the kind, and the marker trait (`crux_core::operation::Notify`, `Request` or `Stream`) that goes with it, so the two can't disagree. Sending an operation with the wrong constructor then fails to compile.

````admonish note
Outputs must be types type generation can emit, which rules out
`std::result::Result`. Where an operation can fail, the convention is a
concrete two-variant enum in the style of `crux_http`'s `HttpResult`:

```rust,ignore
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

If you're upgrading an app written against the older, enum-shaped capabilities, the [Migrating to per-operation types](../guide/migrate-per-operation-types.md) guide is the mechanical version of this section. And if you're upgrading from before `crux_http` 0.19, note that it switched from `http-types` to the standard [`http`](https://docs.rs/http) crate — see [Migrating `crux_http` to native `http` types](../guide/migrate-crux-http.md).

## A simple custom capability: Location

`Render` ships in `crux_core`; `crux_http`, `crux_kv`, and `crux_time` are separate crates Crux publishes. Location services aren't — they work differently enough across platforms that a cross-platform crate would do more harm than good, and they're specific enough that we didn't want to maintain an official one either. So the weather app defines its own.

A capability is two things:

1. A protocol for talking to the shell — an operation type per operation, each with its output.
2. An ergonomic API for the core developer — usually a handful of command-builder functions.

Here's the whole protocol for Location:

```rust
{{#include ../../../examples/weather/shared/src/effects/location/mod.rs}}
```

Two operations, two outputs, and nothing shared between them. `IsLocationEnabled` is answered with a `bool`; `GetLocation` with an `Option<Location>`. Both declare `request`, so the shell answers each exactly once and the compiler rejects `Command::notify_shell(GetLocation)`.

`register(Location)` on `GetLocation` is a type generation hint: the tracer reaches `Option<Location>` from the output, but naming `Location` explicitly guarantees the struct is emitted for the shells even if nothing else references it.

The developer API is equally small:

```rust
{{#include ../../../examples/weather/shared/src/effects/location/command.rs}}
```

Each function issues one operation and hands back its output. There's no narrowing step and no fallback for "the shell answered the other question", because there is no other question this request could be answered with.

Notice the generic signatures: both functions are generic over `Effect` and `Event`. The trait bound `Effect: From<Request<GetLocation>>` says the caller's `Effect` type must be able to wrap *that* request — every `#[effect]`-generated enum implements it for each operation it lists. Bounding per operation rather than per capability is what lets an app carry `KvGet` and `KvSet` without ever hearing about `ListKeys`.

## A richer example: Secret

Location is about as minimal as a capability gets. Secret — storing, fetching, and deleting an API key — has three operations whose outcomes genuinely differ, and it's the clearest illustration of what per-operation outputs buy you.

Fetching a secret either finds it or doesn't. Storing one either succeeds or fails. Deleting one either succeeds or fails. With one shared response enum those six outcomes live in one type, and every call site has to rule out the four that can't happen — which is exactly what this capability used to do, with `unreachable!()` in the arms it had ruled out by hand.

Now each operation names only its own outcomes:

```rust
{{#include ../../../examples/weather/shared/src/effects/secret/mod.rs}}
```

Three operations, three outputs, two variants each. There is no wide `SecretResponse` and no `unreachable!()` anywhere, because a `SecretStoreResponse` is not a possible answer to a `Fetch` — the type says so, and the shell's generated handler method for `Fetch` returns a `SecretFetchResponse` or nothing at all.

The developer API is correspondingly plain:

```rust
{{#include ../../../examples/weather/shared/src/effects/secret/command.rs}}
```

Each builder issues its request and returns the output unchanged. Compare that with what it replaced: a `.map(...)` per builder, matching the wide response down to the narrow one and panicking on the arms that "can't happen".

Using these builders looks no different from the location ones: call `secret::command::fetch(API_KEY_NAME)` and finish with `.then_send(...)` to bind the eventual `SecretFetchResponse` to an event.

## Notifications and streams

Both capabilities above are requests. The notes example's pub/sub capability has one of each of the other two kinds, which makes it the best place to see them.

```rust
{{#include ../../../examples/notes/shared/src/capabilities/pub_sub.rs:operations}}
```

`Publish` is a **notification**: the shell broadcasts the bytes and there is nothing to answer, so its `Output` is `()` and `#[operation(notify)]` takes no `output` argument at all. `Subscribe` is a **stream**: the shell resolves the request once per `Message` that arrives from a peer, for as long as the subscription lives.

The builders differ in the same way:

```rust
{{#include ../../../examples/notes/shared/src/capabilities/pub_sub.rs:builders}}
```

`Command::stream_from_shell` produces a `StreamBuilder`, whose `.then_send` fires an event per item rather than once; `Command::notify_shell` produces a `NotificationBuilder`, which has no output to send anywhere. Try them the other way round and the compiler stops you:

```text
error[E0080]: evaluation panicked: this operation does not declare
RequestKind::Request; send it with notify_shell or stream_from_shell instead
```

```admonish note title="Where that error appears"
In this release the check is a `const` assertion evaluated after
monomorphisation, so it fires on `cargo build`, `cargo test` or
`cargo clippy --all-targets` — but *not* on `cargo check` or in your editor.
The next breaking release moves the kind to an associated type, at which point
it becomes an ordinary trait-bound error you see as you type.
```

The kinds pay off hardest on the shell side. Because each variant's kind is static, [type generation](../part-4/typegen.md) can emit a handler interface where `publish` returns nothing, `subscribe` is handed a sink to send `Message`s into, and a request method returns its output — the shell can't resolve the wrong number of times, because there is no `resolve` for it to call.

## What capabilities provide

Putting it together, a capability gives you two things:

- **A protocol** — one operation type per operation, each declaring its single output and its request kind, which together define the wire format between core and shell.
- **A developer API** — small command-builder functions that speak in convenient Rust types rather than the raw protocol.

In [ports-and-adapters](https://en.wikipedia.org/wiki/Hexagonal_architecture) vocabulary, capabilities are the ports; the shell-side code that actually carries out each operation is the adapter. The core expresses *what* it wants done; the shell decides *how* to do it. Keeping that separation tight is what makes the core portable.

Speaking of the shell — it's time to look at how these operations get carried out on each platform. That's the next chapter.
