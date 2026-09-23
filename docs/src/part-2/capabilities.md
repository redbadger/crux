# Building capabilities

We covered effects and commands in detail, and hinted throughout at capabilities — the developer-friendly APIs you actually use when writing core code. Time to look at them directly, both using them and building our own.

In practice, apps need a fairly limited number of capabilities — typically around seven, almost certainly fewer than ten. The weather app uses six: Render, KeyValue, Http, Location, Secret, and Time. Capabilities are reusable across apps — if you build one that others would benefit from, the Crux team would like to hear about it.

## Using a capability

Capabilities don't return a `Command` directly — they return a command *builder*, which lets you chain behaviour before committing to a specific event. We saw the abstract shape in chapter 5: `Http::get(...).expect_json().build().then_send(Event::ReceivedResponse)`.

The weather app's current-weather fetch shows the same pattern in production code:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/http/weather/mod.rs:fetch}}
```

`Http::get(...)` starts a builder, `.expect_json::<T>()` pins down the response type, `.query(...)` adds URL parameters, `.build()` produces a `RequestBuilder`, and `.map(...)` translates the shell's `Result<Response, HttpError>` into the more convenient `Result<CurrentWeatherResponse, WeatherError>`. The caller finishes it off with `.then_send(SomeEvent)` — `fetch` returns a builder, not a command, so callers can hook it into their own event type.

Note that a 4xx or 5xx response arrives on the **`Err`** side of that `Result`, never as an `Ok(Response)` carrying an error status — see [Handling `crux_http` rejections](../guide/http-rejections.md) for how to read one and how to test it.

That's how a capability gets used. But where do these APIs come from? Let's build one.

## One output per operation

Before we write any code, one rule shapes everything that follows: **an operation type has exactly one output type, and exactly one operation kind.**

The `Operation` trait has always said the first half:

```rust,ignore
// Rust
pub trait Operation: Send + 'static {
    type Output: Send + Unpin + 'static;
}
```

The trouble is that it used to be conventional to implement it on a *coarse enum* — one operation type with five variants, one output type with five variants — and then the trait's promise stops being true in practice. Any response variant is a well-formed answer to any request variant, as far as the type system and the deserializer are concerned. So the capability has to check at runtime, and every capability author has to decide what to do when the check fails: panic, invent an error, or quietly ignore it. `crux_kv` and `crux_time` both used to panic.

So: one type per operation. Each carries its own output, and the wrong answer stops being expressible.

The second half — the operation kind — is the same idea applied to *how many times* the shell answers:

- **notify** — the shell is told, and never answers. `Output` is `()`.
- **request** — the shell answers exactly once, with the operation's `Output`.
- **stream** — the shell answers any number of times, each with an `Output`.

That used to be decided by which `Command` constructor you called, so the same operation could be notified in one place and streamed in another. Declaring it on the type instead means the compiler can hold you to it, and — more usefully — it means [type generation](../part-4/typegen.md) can tell the *shell* how many times to resolve each effect, which it previously had to learn by reading Rust source.

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

If you're upgrading an app written against the older, enum-shaped capabilities, the [Migrating to per-operation types](../guide/migrate-per-operation-types.md) guide is the mechanical version of this section. And if you're upgrading from before `crux_http` 0.19, note that it switched from `http-types` to the standard [`http`](https://docs.rs/http) crate — see [Migrating `crux_http` to native `http` types](../guide/migrate-crux-http.md).

## A simple custom capability: Location

`Render` ships in `crux_core`; `crux_http`, `crux_kv`, and `crux_time` are separate crates Crux publishes. Location services aren't — they work differently enough across platforms that a cross-platform crate would do more harm than good, and they're specific enough that we didn't want to maintain an official one either. So the weather app defines its own.

A capability is two things:

1. A protocol for talking to the shell — an operation type per operation, each with its output.
2. An ergonomic API for the core developer — usually a handful of command-builder functions.

Here's the whole protocol for Location:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/location/mod.rs}}
```

Two operations, two outputs, and nothing shared between them. `IsLocationEnabled` is answered with a `bool`; `GetLocation` with an `Option<Location>`. Both declare `request`, so the shell answers each exactly once and the compiler rejects `Command::notify_shell(GetLocation)`.

`Location` itself needs no mention anywhere: type generation walks each output's shape and emits every type it finds, so `Option<Location>` is enough for the shells to get a `Location`.

The developer API is equally small:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/location/command.rs}}
```

Each function issues one operation and hands back its output. There's no narrowing step and no fallback for "the shell answered the other question", because there is no other question this request could be answered with.

Notice the generic signatures: both functions are generic over `Effect` and `Event`. The trait bound `Effect: From<Request<GetLocation>>` says the caller's `Effect` type must be able to wrap *that* request — every `#[effect]`-generated enum implements it for each operation it lists. Bounding per operation rather than per capability is what lets an app carry `KvGet` and `KvSet` without ever hearing about `ListKeys`.

## A richer example: Secret

Location is about as minimal as a capability gets. Secret — storing, fetching, and deleting an API key — has three operations whose outcomes genuinely differ, and it's the clearest illustration of what per-operation outputs buy you.

Fetching a secret either finds it or doesn't. Storing one either succeeds or fails. Deleting one either succeeds or fails. With one shared response enum those six outcomes live in one type, and every call site has to rule out the four that can't happen — which is exactly what this capability used to do, with `unreachable!()` in the arms it had ruled out by hand.

Now each operation names only its own outcomes:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/secret/mod.rs}}
```

Three operations, three outputs, two variants each. There is no wide `SecretResponse` and no `unreachable!()` anywhere, because a `SecretStoreResponse` is not a possible answer to a `FetchSecret` — the type says so, and the shell's generated handler method for `FetchSecret` returns a `SecretFetchResponse` or nothing at all.

The operation names carry the capability's name for a reason that has nothing to do with Rust: type generation puts every operation of every capability into one namespace on the shell side, so a bare `Delete` here would clash with any other capability or app operation called `Delete`, and type generation refuses two shared types that generate the same name. An app-defined operation with a generic name is worth prefixing from the start.

The developer API is correspondingly plain:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/secret/command.rs}}
```

Each builder issues its request and returns the output unchanged. Compare that with what it replaced: a `.map(...)` per builder, matching the wide response down to the narrow one and panicking on the arms that "can't happen".

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
In this release the check is a `const` assertion evaluated after
monomorphisation, so it fires on `cargo build`, `cargo test` or
`cargo clippy --all-targets` — but *not* on `cargo check` or in your editor.
The next breaking release moves the kind to an associated type, at which point
it becomes an ordinary trait-bound error you see as you type.
```

The kinds pay off hardest on the shell side. Because each variant's kind is static, [type generation](../part-4/typegen.md) can emit a handler interface where `publish` returns nothing, `subscribe` is handed a sink to send `Message`s into, and a request method returns its output — the shell can't resolve the wrong number of times, because there is no `resolve` for it to call.

## What capabilities provide

Putting it together, a capability gives you two things:

- **A protocol** — one operation type per operation, each declaring its single output and its kind, which together define the wire format between core and shell.
- **A developer API** — small command-builder functions that speak in convenient Rust types rather than the raw protocol.

In [ports-and-adapters](https://en.wikipedia.org/wiki/Hexagonal_architecture) vocabulary, capabilities are the ports; the shell-side code that actually carries out each operation is the adapter. The core expresses *what* it wants done; the shell decides *how* to do it. Keeping that separation tight is what makes the core portable.

## Shipping the shell side too

A published capability can go one step further and carry a reference adapter with it: a Swift, Kotlin, TypeScript and C# implementation of its operations, embedded in the crate as source. `crux_http`, `crux_kv` and `crux_time` all do. An app that registers one in its codegen binary gets the file emitted into its generated package, and its handler delegates to it one line per operation — see [Shipped shell handlers](../part-4/typegen.md#shipped-shell-handlers) for that side of the story. This section is about what the capability author writes.

The rules of a protocol belong to whoever defines it. How a `URLError` maps onto `HttpResult`, what a timer answers with when it fires and what it answers when it is cleared first: these are decisions the capability made, and every shell that re-derives them from documentation can get them subtly wrong. Shipping the implementation puts them in one place, next to the Rust that defines the operations, where a change to one is reviewed with the other. What it does *not* decide is whether a given app uses it — a bank with a hardened HTTP stack keeps its own — so the shipped file is a plain type in the app's module, and nothing generated calls it.

The declaration is a `static` behind the crate's `facet_typegen` feature, so the source text is compiled into the app's codegen binary and never into its core:

```rust,ignore
// Rust — crux_time/src/shell.rs, re-exported from lib.rs
#[cfg(feature = "facet_typegen")]
pub static TIME: ShellHandler = ShellHandler::new("Time")
    .types(register_types)
    .swift(ShellSource::stdlib(include_str!("../shell/swift/Time.swift")))
    .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Time.kt")))
    .typescript(ShellSource::stdlib(include_str!("../shell/typescript/time.ts")))
    .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Time.cs")));
```

`name` names the file the app receives (`Time.swift`) and the protocol inside it. `types` names a function that registers every operation and output the sources mention — the shipped file implements the whole capability, so all of its types have to be generated even for an app whose `Effect` carries only some of them. A language you do not name is not shipped; the app implements those methods itself, as it would have without you.

Each file declares a protocol called `<Name>Handler` (`I<Name>Handler` in C#) with one method per operation, taking the operation and returning exactly what the generated `EffectHandler` method for that operation returns — the output for a request, nothing for a notification, an `EffectSink` for a stream — and at least one implementation of it. Matching the shapes is what makes the app's delegation a single expression:

```swift
// Swift — crux_time/shell/swift/Time.swift
public protocol TimeHandler: Sendable {
    func now(_ operation: Now) async -> Instant
    func notifyAt(_ operation: NotifyAt) async -> TimerId
    func notifyAfter(_ operation: NotifyAfter) async -> TimerId
    func clear(_ operation: ClearTimer) async -> TimerId
}

public final class TaskTimeHandler: TimeHandler, @unchecked Sendable {
    /* a table of running timers, keyed by id, guarded by a lock */
}
```

The source refers to the generated types unqualified — `Now`, `Instant`, `TimerId` — because it is emitted into the same module they are. It carries its own imports after the module header type generation prepends, and nothing else at module scope under a name a generated type could take; `<Name>Handler` is reserved for it.

A few constraints keep a shipped file safe to drop into any app:

- **Standard library only.** Foundation, the JDK and `kotlinx-coroutines-core`, the browser or Node globals, the BCL — nothing the generated module does not already require. The Kotlin source is JVM, not Android: the generated package is a `kotlin("jvm")` library, so `android.*` would not compile there. A capability that truly needs a library adds it with `ShellSource::stdlib(..).dependencies(&[..])`, and every app that registers the handler then pays for it, so the bar is high.
- **Say where the lowest common denominator ends.** Writing to the standard library means a platform with something better — `DataStore` or Room on Android, Cronet for HTTP, a database anywhere — beats what you shipped. That is fine: the shell conforms its own type to your protocol and provides that instead. What is not fine is leaving an app to discover it. `crux_kv`'s file store documents that it caches nothing and spans no more than one key; its `UserDefaults` implementation documents that documents and caches belong in a file.
- **The same source on every platform that runs the language.** Swift's standard library differs between Apple platforms and corelibs-foundation: `URLSession` and its companions live in `FoundationNetworking` there, so a source that uses them needs `#if canImport(FoundationNetworking)`. The `tests/shell_source.rs` below builds Swift on Linux in CI, which is how you find out.
- **Locks, not actors, in Swift.** The generated operation and output types are not `Sendable`, so an actor cannot return one across its isolation boundary. A shipped implementation that holds state guards it with a lock and declares itself `@unchecked Sendable`, as `TaskTimeHandler` does above.
- **Configuration through the initialiser.** A shared instance for the plain path (`URLSessionHttpHandler.shared`), an initialiser for the configured one (`URLSessionHttpHandler(session:)`), no globals to mutate.
- **Quiet.** No logging through an app-specific logger. Stay silent or expose a hook on the protocol.
- **Compiled somewhere on every change.** `cargo test` cannot compile Swift. The bundled capabilities are compiled by the weather and notes example shells and by a `tests/shell_source.rs` in each crate, which generates a package for a small app and runs `dotnet build` or `tsc` when the toolchain is present. Give your own capability the same.

Put the files under `shell/<language>/` beside `src/`, and make sure `Cargo.toml` packages them if it lists what to `include`.

Speaking of the shell — it's time to look at how these operations get carried out on each platform. That's the next chapter.
