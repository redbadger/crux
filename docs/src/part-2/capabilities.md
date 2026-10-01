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

The `Operation` trait has always said the first half:

```rust,ignore
// Rust
pub trait Operation: Send + 'static {
    type Output: Send + Unpin + 'static;
}
```

The trouble is that it used to be conventional to implement it on a *coarse enum* (one operation type with five variants, one output type with five variants), and then the trait's promise stops being true in practice. Any response variant is a well-formed answer to any request variant, as far as the type system and the deserializer are concerned. So the capability has to check at runtime, and every capability author has to decide what to do when the check fails: panic, invent an error, or quietly ignore it. `crux_kv` and `crux_time` both used to panic.

So: one type per operation. Each carries its own output, and the wrong answer stops being expressible.

The second half, the operation kind, is the same idea applied to *how many times* the shell answers:

- **notify**: the shell is told, and never answers. `Output` is `()`.
- **request**: the shell answers exactly once, with the operation's `Output`.
- **stream**: the shell answers any number of times, each with an `Output`.

That used to be decided by which `Command` constructor you called, so the same operation could be notified in one place and streamed in another. Declaring it on the type instead means the compiler can hold you to it and, more usefully, it means [type generation](../part-4/typegen.md) can tell the *shell* how many times to resolve each effect, which it previously had to learn by reading Rust source.

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

If you're upgrading an app written against the older, enum-shaped capabilities, the [Migrating to per-operation types](../guide/migrate-per-operation-types.md) guide is the mechanical version of this section. And if you're upgrading from before `crux_http` 0.19, note that it switched from `http-types` to the standard [`http`](https://docs.rs/http) crate (see [Migrating `crux_http` to native `http` types](../guide/migrate-crux-http.md)).

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

Two operations, two outputs, and nothing shared between them. `IsLocationEnabled` is answered with a `bool`; `GetLocation` with an `Option<Location>`. Both declare `request`, so the shell answers each exactly once and the compiler rejects `Command::notify_shell(GetLocation)`.

`Location` itself needs no mention anywhere: type generation walks each output's shape and emits every type it finds, so `Option<Location>` is enough for the shells to get a `Location`.

The developer API is equally small:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/location/command.rs}}
```

Each function issues one operation and hands back its output. There's no narrowing step and no fallback for "the shell answered the other question", because there is no other question this request could be answered with.

Notice the generic signatures: both functions are generic over `Effect` and `Event`. The trait bound `Effect: From<Request<GetLocation>>` says the caller's `Effect` type must be able to wrap *that* request. Every `#[effect]`-generated enum implements it for each operation it lists. Bounding per operation rather than per capability is what lets an app carry `KvGet` and `KvSet` without ever hearing about `ListKeys`.

## A richer example: Secret

Location is about as minimal as a capability gets. Secret (storing, fetching, and deleting an API key) has three operations whose outcomes genuinely differ, and it's the clearest illustration of what per-operation outputs buy you.

Fetching a secret either finds it or doesn't. Storing one either succeeds or fails. Deleting one either succeeds or fails. With one shared response enum those six outcomes live in one type, and every call site has to rule out the four that can't happen, which is exactly what this capability used to do, with `unreachable!()` in the arms it had ruled out by hand.

Now each operation names only its own outcomes:

```rust
// Rust
{{#include ../../../examples/weather/shared/src/effects/secret/mod.rs}}
```

Three operations, three outputs, two variants each. There is no wide `SecretResponse` and no `unreachable!()` anywhere, because a `SecretStoreResponse` is not a possible answer to a `FetchSecret`. The type says so, and the shell's generated handler method for `FetchSecret` returns a `SecretFetchResponse` or nothing at all.

The operation names carry the capability's name for a reason that has nothing to do with Rust: type generation puts every operation of every capability into one namespace on the shell side, so a bare `Delete` here would clash with any other capability or app operation called `Delete`. Type generation rejects two shared types that generate the same name. An app-defined operation with a generic name is worth prefixing from the start.

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
The check is a `const` assertion evaluated after monomorphisation, so it fires
on `cargo build`, `cargo test` or `cargo clippy --all-targets`, but *not* on
`cargo check` or in your editor. The
[operation-kind traits RFC](../rfcs/operation-kind-traits.md) proposes making
the kind an associated type of `Operation`, which would turn this into an
ordinary trait-bound error you see as you type.
```

The kinds pay off hardest on the shell side. Because each variant's kind is static, [type generation](../part-4/typegen.md) can emit a handler interface where `publish` returns nothing, `subscribe` is handed a sink to send `Message`s into, and a request method returns its output. The shell can't resolve the wrong number of times, because there is no `resolve` for it to call.

## Shipping a shell protocol

Every shell that uses a capability implements its operations, and it must follow the same rules. For example, a 4xx status is an answer and not an error. A capability can state these rules once, as a shell protocol: the handler interface that a shell implements, in Swift, Kotlin, TypeScript and C#. [Type generation](../part-4/typegen.md#shell-protocols) emits the shell protocol for every app that uses an operation of the capability. The app does not register it.

To ship a shell protocol, write one source file for each language. Put each file at `shell/<language>/` in the capability crate, for example `shell/swift/KeyValue.swift`. A file holds the interface, with one method for each operation, and a doc comment that gives the rules. A file must use only the standard library of its language, and must put its imports at the top. Do not name an implementation or show how to construct one, because the shell protocol must be complete without an implementation.

Then declare the shell protocol in a `shell` module. Put the items inside `crux_core::__crux_core_facet_typegen_items!`, so that they exist only when the `facet_typegen` feature of `crux_core` is on:

```rust,ignore
crux_core::__crux_core_facet_typegen_items! {
    use crux_core::{
        capability::Operation as _,
        type_generation::facet::{ShellProtocol, TypeGenError, TypeRegistry},
    };

    pub static PROTOCOL: ShellProtocol = ShellProtocol::new("KeyValue")
        .rust_types(register_types)
        .swift(include_str!("../shell/swift/KeyValue.swift"), &["KeyValueHandler"])
        .kotlin(include_str!("../shell/kotlin/KeyValue.kt"), &["KeyValueHandler"])
        .typescript(include_str!("../shell/typescript/keyValue.ts"), &["KeyValueHandler"])
        .csharp(include_str!("../shell/csharp/KeyValue.cs"), &["IKeyValueHandler"]);

    /// Registers every operation of the capability, and its output.
    pub fn register_types(registry: &mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError> {
        GetValue::register_types_facet(registry)?;
        SetValue::register_types_facet(registry)
    }
}
```

Keep the `shell` module itself unconditional, and put only its items inside the macro.

The name (`"KeyValue"`) names the file that type generation writes. The second argument of each language is the list of top-level names that the file declares. Type generation checks each name for a clash with the generated code and with the app's types.

`register_types` registers every operation of the capability, because a shell implements the whole shell protocol. Thus the app's generated code has every type that the shell protocol names, including the types of the operations that the app does not send. Type generation rejects an app type that generates under one of those names, and the error names both Rust types.

Last, name the shell protocol on every operation, with `protocol =`:

```rust,ignore
#[derive(Operation, Facet, Serialize, Deserialize, Clone, Debug)]
#[operation(request, output = ValueResult, protocol = crate::shell::PROTOCOL)]
pub struct GetValue { pub key: String }
```

The derive then makes the operation register the shell protocol when type generation registers the operation. The derived code is inside the same macro, so the shell protocol follows the `facet_typegen` feature of `crux_core`. An app that turns on `crux_core/facet_typegen` gets the shell protocol, and it needs no feature of the capability crate for it. The capability crate must depend on `facet` and `serde`.

To test the shell protocol, generate the code for a small app that uses some of the operations. Then compile the result with `swift build`, `kotlinc`, `tsc` and `dotnet build`. The `tests/shell_protocol.rs` file in `crux_kv` does this. The `tests/protocol_follows_crux_core.rs` file in `crux_kv` checks that an operation registers the shell protocol when only the `facet_typegen` feature of `crux_core` is on. If a toolchain is not on `PATH`, the test prints a message and does not run that check. If `CRUX_REQUIRE_SHELL_TOOLCHAINS` is set, a missing toolchain is a failure.

## What capabilities provide

Putting it together, a capability gives you two things:

- **A protocol**: one operation type per operation, each declaring its single output and its kind, which together define the wire format between core and shell.
- **A developer API**: small command-builder functions that speak in convenient Rust types rather than the raw protocol.

In [ports-and-adapters](https://en.wikipedia.org/wiki/Hexagonal_architecture) vocabulary, capabilities are the ports; the shell-side code that actually carries out each operation is the adapter. The core expresses *what* it wants done; the shell decides *how* to do it. Keeping that separation tight is what makes the core portable.

## Shipping an implementation

A capability can also ship an implementation of its shell protocol in each language, as source embedded in the crate. `crux_http`, `crux_kv` and `crux_time` all do. The shell protocol is automatic, but an implementation is opt-in: an app gets one only if it registers it in its codegen binary. The app's handler then delegates to it in one line for each operation. See [Shell extensions](../part-4/typegen.md#shell-extensions) for the app's side. This section is about what the capability author writes.

The shell protocol gives the rules. An implementation applies them to one platform client, for example when it decides which `URLError` is a timeout. If the capability ships the implementation, these decisions are in one place, next to the Rust that defines the operations, and a change to one is reviewed with the other. The capability does not decide if an app uses the implementation. For example, a bank with a hardened HTTP stack keeps its own. So the shipped file is a plain type in the app's module, and no generated code calls it.

An implementation is a shell extension. Declare a `static` for each language, in a module for that language, behind the crate's `facet_typegen` feature. Thus the source text is compiled into the app's codegen binary and never into its core:

```rust,ignore
// Rust: crux_time/src/shell.rs
#[cfg(feature = "facet_typegen")]
pub mod swift {
    use crux_core::type_generation::facet::{Platform, ShellExtension};
    use include_dir::include_dir;

    pub static TASK: ShellExtension = ShellExtension::swift(
        "Time",
        include_dir!("$CARGO_MANIFEST_DIR/shell/swift/task"),
    )
    .platforms(&[Platform::Apple, Platform::Linux])
    .protocol(&super::PROTOCOL)
    .foreign_types(&["TaskTimeHandler"]);
}

// `kotlin::COROUTINE`, `typescript::TIMEOUT` and `csharp::TASK` are the same.
```

`include_dir!` expands to a path in the `include_dir` crate, so the capability depends on `include_dir` itself, as an optional dependency that its `facet_typegen` feature turns on. `name` is the name of the capability (`Time`), and the extensions of all languages share it. The directory holds the source. Crux emits each file in its `common/` directory, and each file in the directory of the platform for which the app generates code, as a file of the same name (`TaskTime.swift`). Name each file after the implementation, because a file must not have the same name as the file of the shell protocol (`Time.swift`). `platforms` lists the platforms on which the source runs. `protocol` names the shell protocol that the source implements. When the app registers the extension, crux also registers that shell protocol, so the implementation always arrives with its shell protocol and with every type that it names. A language for which you ship no extension gets only the shell protocol, and the app implements it.

Each file declares at least one implementation of the shell protocol. The file does not declare the shell protocol again, because the shell protocol comes with the operations:

```swift
// Swift: crux_time/shell/swift/task/common/TaskTime.swift
public final class TaskTimeHandler: TimeHandler, @unchecked Sendable {
    /* a table of running timers, keyed by id, guarded by a lock */
}
```

The source refers to the generated types such as `Now`, `Instant` and `TimerId` unqualified, because type generation emits it into the same module as those types. It carries its own imports, after the module header that type generation writes first. `foreign_types` lists every name that the extension's sources declare at module scope: the implementations, and in TypeScript any top-level helper or constant, exported or not, because the source is appended to the module file. Do not list the names of the shell protocol. When the app builds its registry, crux checks each of those names against the generated shell API, the imports of the serialization runtime, the app's root types and namespaces, the names that extensions of other capabilities declare, and the names that the shell protocols declare. Thus a clash is a generation error, not a module that fails to compile. Keep the list complete when you add a declaration. A test can check the list. It scans the sources for module-scope declarations and compares them with `foreign_types`. The bundled crates each have one, in `tests/shell_foreign_types.rs`.

A few constraints keep a shipped file safe to drop into any app:

- **Standard library only.** Use Foundation, the JDK and `kotlinx-coroutines-core`, the browser or Node globals, or the BCL, and nothing that the generated module does not already require. The Kotlin source in `common/` is JVM, not Android: the generated package for the JVM is a `kotlin("jvm")` library, so `android.*` would not compile there. Code that needs `android.*` goes in `android/`. A capability that truly needs a library adds it with `.dependencies(&[..])`, and every app that registers the extension then pays for it, so the bar is high.
- **Say where the lowest common denominator ends.** A platform with something better than the standard library (`DataStore` or Room on Android, Cronet for HTTP, a database anywhere) beats what you shipped. That is fine: the shell implements your shell protocol itself, and registers no extension. What is not fine is to leave an app to discover it. If the right store is different on each platform, ship less. `crux_kv` ships only an in-memory store, for tests, previews and a shell with no storage. A durable store is the app's choice (`UserDefaults`, `DataStore` or `localStorage`, for example), so the app implements `KeyValueHandler` over it. The weather and notes examples each have one in the app's own code. The handlers of `crux_time` document how precise `now` and the timers are, because that varies by platform.
- **A file for each platform, not a compiler directive.** Swift's standard library differs between Apple platforms and corelibs-foundation. On Linux, `URLSession` and its companions are in `FoundationNetworking`, which Apple platforms do not have. So `crux_http` puts its `URLSessionHttpHandler` in two files, `apple/URLSession.swift` and `linux/URLSession.swift`. The two files differ only in their imports. A test in `tests/shell_url_session.rs` compares the two files, so that a change to one file must also go into the other. The `tests/shell_source.rs` file builds the Swift for the platform of the machine, which is Linux in CI. A shell protocol has no platform directories, because it is the same on every platform.
- **Locks, not actors, in Swift.** The generated operation and output types are not `Sendable`, so an actor cannot return one across its isolation boundary. A shipped implementation that holds state guards it with a lock and declares itself `@unchecked Sendable`, as `TaskTimeHandler` does above.
- **Configuration through the initialiser.** Where there is a sensible default, give an implementation an initialiser with no arguments, which uses the platform's shared client where there is one (`URLSessionHttpHandler()` sends with `URLSession.shared`). Always give it one that takes the configuration (`URLSessionHttpHandler(session:)`). Ship no shared instance and no globals to mutate: an app has one core, so a shared instance gives nothing, and the shell constructs what it holds.
- **Say where the state lives.** For `crux_http`, the state is the cookies. `URLSessionHttpHandler()` keeps them in the process-wide `HTTPCookieStorage.shared`, and `new HttpClientHttpHandler()` keeps them in the one static `HttpClient` that every handler made that way shares, so both are process-wide. `UrlConnectionHttpHandler()` keeps them in an `InMemoryCookieJar` of that handler's own, so two handlers have two sessions. `createFetchHttpHandler()` leaves them to the runtime's `fetch`: in a browser that is the browser's cookie store, for same-origin requests, because the handler sets no `credentials`, and under Node there are none.
- **Quiet.** Do not log through an app-specific logger. Stay silent, or give the implementation a hook.
- **Compiled somewhere on every change.** `cargo test` cannot compile Swift. A `tests/shell_source.rs` in each bundled crate generates a package for a small app and builds it with `swift build`, `kotlinc`, `tsc` and `dotnet build`. If a toolchain is missing, the test prints a skip. CI sets `CRUX_REQUIRE_SHELL_TOOLCHAINS`, which turns the skip into a failure. The weather and notes example shells also compile the HTTP and timer implementations. Give your own capability the same.

Put the files that every platform gets under `shell/<language>/<implementation>/common/`, beside the file of the shell protocol. Put the files for one platform only in a directory for that platform, beside `common/`. If `Cargo.toml` lists what to `include`, make sure that it packages these files.

Speaking of the shell, it's time to look at how these operations get carried out on each platform. That's the next chapter.
