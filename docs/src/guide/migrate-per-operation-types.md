# Migrating to per-operation types

From `crux_core` 0.21, an operation is a **type**, not a variant: one type per
operation, each declaring the single output it is answered with and how many
times the shell resolves it. `crux_kv` 0.15 and `crux_time` 0.19 ship
per-operation APIs alongside their old enum ones, and type generation hands the
declared request kinds to shells as a typed handler API.

Nothing is removed in this release. The enum APIs still work, they are
`#[deprecated]` with a replacement named in the warning, and you can migrate one
call at a time. The next breaking release removes them — see
[Coming in the breaking release](#coming-in-the-breaking-release) at the bottom,
and the [RFC](../rfcs/per-operation-types.md) for the design and its reasoning.

---

## Quick checklist

If your app doesn't use `crux_kv` or `crux_time` and doesn't define its own
capabilities, it compiles unchanged and there is nothing to do; the generated
handler API is additive, so your shells keep working too.

Otherwise, in this order:

1. **Your own capabilities** — one struct per operation with
   `#[derive(Operation)]`, and one output type per operation instead of a shared
   response enum.
2. **`crux_kv` and `crux_time`** — swap `KeyValue` for `KeyValueStore` and `Time`
   for `Clock`, and list the operations you use in your `Effect` enum.
3. **Your `Effect` enum** — one variant per operation, which renames the
   generated `is_` / `into_` / `expect_*` test helpers.
4. **Regenerate your shells** and let the generated `Core` drive the loop, or
   adopt just the generated `EffectHandler`, or widen the match you already
   have.
5. **Check the [traps](#traps-worth-knowing-about)** — cleared timers, Swift
   actor isolation, and `Set` name collisions.

---

## Declaring an operation

There are three forms, one per request kind. Use `#[derive(Operation)]`, which
writes the `Operation` implementation, its `Output`, the kind and the matching
marker trait so the three cannot disagree:

```rust
use crux_core::macros::Operation;
use facet::Facet;
use serde::{Deserialize, Serialize};

/// notify — the shell is told and never answers. `Output` is `()`, and
/// declaring an `output` is an error.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

/// request — answered exactly once, with the declared output.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = ValueResult)]
pub struct Get {
    pub key: String,
}

/// stream — answered any number of times, each with the declared output.
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(stream, output = Message)]
pub struct Subscribe;
```

Structs only, of any shape — named, tuple or unit. Generics and `where` clauses
pass through. `output` accepts an unquoted generic type, so
`output = Option<Location>` works.

`Command::notify_shell` then only accepts a `notify` operation,
`request_from_shell` a `request`, and `stream_from_shell` a `stream`:

```text
error[E0080]: evaluation panicked: this operation does not declare
RequestKind::Request; send it with notify_shell or stream_from_shell instead
```

```admonish note title="Where that error appears"
In this release the check is a `const` assertion evaluated after
monomorphisation, so it fires on `cargo build`, `cargo test` or
`cargo clippy --all-targets` — but not on `cargo check` or in your editor. The
breaking release turns it into an ordinary trait-bound error.
```

An operation that declares nothing — a hand-written `impl Operation` with no
`KIND` — keeps working exactly as before, with any constructor. You do not have
to migrate your own capabilities to take `crux_core` 0.21.

### By hand

If you'd rather not use the derive, declare the kind and the marker together:

```rust,ignore
impl Operation for Get {
    type Output = ValueResult;
    const KIND: Option<RequestKind> = Some(RequestKind::Request);
}

impl crux_core::operation::Request for Get {}
```

Import the module, not the items — `operation::Request` reads unambiguously
where a bare `Request` collides with `crux_core::Request<Op>`.

---

## Replacing a response enum

This is the bulk of the work for a hand-written capability, and the weather
example's Secret capability is the clearest case. Three operations shared one
six-variant response, so every call site had to rule out the four variants that
could not apply to it:

```rust,ignore
// Before
pub enum SecretRequest {
    Fetch(String),
    Store(String, String),
    Delete(String),
}

pub enum SecretResponse {
    Missing(String),
    Fetched(String),
    Stored(String),
    StoreError(String),
    Deleted(String),
    DeleteError(String),
}

impl Operation for SecretRequest {
    type Output = SecretResponse;
}

// … and in every builder, a narrowing `.map`:
pub fn fetch<Ef, Ev>(key: impl Into<String>)
    -> RequestBuilder<Ef, Ev, impl Future<Output = SecretFetchResponse>>
{
    Command::request_from_shell(SecretRequest::Fetch(key.into())).map(|response| {
        match response {
            SecretResponse::Missing(key) => SecretFetchResponse::Missing(key),
            SecretResponse::Fetched(value) => SecretFetchResponse::Fetched(value),
            _ => unreachable!("the shell only answers a Fetch with Missing or Fetched"),
        }
    })
}
```

After, each operation is its own type and the narrow response types it already
had become its output:

```rust,ignore
// After
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = SecretFetchResponse)]
pub struct Fetch(pub String);

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = SecretStoreResponse)]
pub struct Store(pub String, pub String);

#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = SecretDeleteResponse)]
pub struct Delete(pub String);

pub fn fetch<Ef, Ev>(key: impl Into<String>)
    -> RequestBuilder<Ef, Ev, impl Future<Output = SecretFetchResponse>>
where
    Ef: From<Request<Fetch>> + Send + 'static,
{
    Command::request_from_shell(Fetch(key.into()))
}
```

The wide `SecretResponse` is deleted, every `unreachable!()` with it, and the
`.map` narrowing goes — the output *is* the narrow type. On the shell side the
`switch`/`when` over the operation enum goes the same way: each generated handler
method takes one operation and returns one output.

Three things to watch:

- **Bound per operation, not per capability.** `Ef: From<Request<Fetch>>` rather
  than `Ef: From<Request<SecretRequest>>`, so an app that only fetches never has
  to carry `Store` and `Delete`.
- **Outputs must be types type generation can emit**, which rules out
  `std::result::Result`. Where an operation can fail, use a concrete two-variant
  enum in the style of `crux_http`'s `HttpResult` — `Ok(T) | Err(E)` — and keep a
  `From` impl if your builders hand a `Result` alias back to app code.
- **A notification has no output.** If a variant was only ever notified, its type
  becomes `#[operation(notify)]` with no `output` at all.

---

## `crux_kv`

`KeyValueStore` has the same five methods as `KeyValue`, with the same
signatures and the same `DataResult` / `StatusResult` / `ListResult` return
types, so app code barely changes. What changes is the `Effect` enum.

```rust,ignore
// Before
use crux_kv::{KeyValue, KeyValueOperation, error::KeyValueError};

#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    KeyValue(KeyValueOperation),
}

KeyValue::get("note").then_send(Event::Load)
```

```rust,ignore
// After
use crux_kv::{KeyValueStore, error::KeyValueError, operation as kv};

#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    KvGet(kv::Get),
    KvSet(kv::Set),
}

KeyValueStore::get("note").then_send(Event::Load)
```

The operations and their outputs:

| Operation | Fields | Output | Kind |
| --- | --- | --- | --- |
| `operation::Get` | `key: String` | `ValueResult` | request |
| `operation::Set` | `key: String, value: Vec<u8>` | `ValueResult` | request |
| `operation::Delete` | `key: String` | `ValueResult` | request |
| `operation::Exists` | `key: String` | `BoolResult` | request |
| `operation::ListKeys` | `prefix: String, cursor: u64` | `KeysResult` | request |

```rust,ignore
pub enum ValueResult { Ok(Value), Err(KeyValueError) }
pub enum BoolResult  { Ok(bool),  Err(KeyValueError) }
pub enum KeysResult  { Ok(Keys),  Err(KeyValueError) }

pub struct Keys { pub keys: Vec<String>, pub next_cursor: u64 }
```

`KeyValueError`, `Value`, `DataResult`, `StatusResult` and `ListResult` are
shared by both APIs and are not deprecated. Each new output converts to and from
the `Result` alias in both directions, so a shell that already speaks one API can
serve the other while you migrate.

The wire shapes change, so shells need regenerating: `KeyValueResponse::Get {
value }` becomes `ValueResult::Ok(value)`, and there is no longer an operation
enum to switch on.

---

## `crux_time`

`Clock` mirrors `Time`'s three methods — `now`, `notify_at`, `notify_after` —
with the same signatures, and shares `TimerHandle`, `CompletedTimerHandle`,
`TimerOutcome`, `TimerId`, `Instant` and `Duration` with it.

```rust,ignore
// Before
use crux_time::{TimeRequest, command::{Time, TimerHandle, TimerOutcome}};

#[effect(facet_typegen)]
pub enum Effect {
    Time(TimeRequest),
}

let (notify_after, handle) = Time::notify_after(duration);
```

```rust,ignore
// After
use crux_time::{Clock, TimerHandle, TimerOutcome, operation as time};

#[effect(facet_typegen)]
pub enum Effect {
    TimeNotifyAfter(time::NotifyAfter),
    TimeClear(time::Clear),
}

let (notify_after, handle) = Clock::notify_after(duration);
```

| Operation | Fields | Output | Kind |
| --- | --- | --- | --- |
| `operation::Now` | — | `Instant` | request |
| `operation::NotifyAt` | `id: TimerId, instant: Instant` | `TimerId` | request |
| `operation::NotifyAfter` | `id: TimerId, duration: Duration` | `TimerId` | request |
| `operation::Clear` | `id: TimerId` | `()` | **notify** |

Two differences to carry across:

- **A `NotifyAt` / `NotifyAfter` is answered with the bare `TimerId`** it was
  given, rather than a `TimeResponse::DurationElapsed { id }`. The core still
  checks it against the timer it started.
- **`Clear` is a notification.** `TimerHandle::clear` sends an `operation::Clear`
  and the timer's future resolves with `TimerOutcome::Cleared`
  **immediately** — it no longer waits for the `TimeResponse::Cleared`
  acknowledgement `Time` waits for. A shell serving `Clock` has no response to
  send for a `Clear`, and must not send one; see the trap below.

Note that if you list `TimeClear` in your `Effect` but not `TimeNotifyAfter`,
nothing will compile — `Clock::notify_after` needs both, since clearing is part
of the handle it returns.

---

## The `Effect` enum, and the helpers it generates

The convention the examples follow:

- **Third-party operations** get a capability prefix: `KvGet`, `KvSet`,
  `TimeNotifyAfter`, `TimeClear`. Without it, `Get` and `Set` on their own read
  as if they belonged to the app, and `Set` collides with a standard library type
  in three of the four shell languages.
- **Your own operations** are verb-first and unprefixed: `Publish`, `Subscribe`,
  `IsLocationEnabled`, `GetLocation`, `FetchSecret`, `StoreSecret`,
  `DeleteSecret`.
- **List only the operations the app uses.** Bounds are per operation, so an app
  that never lists keys is never asked to serve `ListKeys` — and neither is its
  shell.

Renaming variants renames the test helpers `#[effect]` generates from them, which
is usually the largest mechanical diff in an app's test suite:

```rust,ignore
// Before                              // After
effects.next().unwrap()                effects.next().unwrap()
    .expect_key_value()                    .expect_kv_get()
Effect::is_key_value                   Effect::is_kv_get
cmd.expect_secret_with(..)             cmd.expect_fetch_secret_with(..)
```

The set is `is_<variant>`, `into_<variant>`, `expect_<variant>`,
`expect_<variant>_with` and `expect_only_<variant>`, each in snake_case from the
variant name.

---

## Regenerating shells and adopting the handler API

Run your `typegen` recipe. Alongside the generated `Effect`, you now get a
`RequestKind` accessor, an `EffectHandler` protocol/interface with one method per
variant, and an `EffectDispatcher` that resolves each request for you — never for
a notification, once for a request, once per sink item for a stream. See
[Type generation](../part-4/typegen.md#request-kinds-and-the-effect-handler-api)
for the exact shapes in each language.

Adopting it is optional. **Matching on `Effect` and calling `resolve` by hand
keeps working**, and is the right choice for Rust shells — see
[keeping a flat match](#keeping-a-flat-match).

### Letting the generated Core own the loop

You also get a `Core` class and a `CoreBridge` protocol (`ICoreBridge` in C#).
`Core` is the loop you used to write around the dispatcher — serialize the event,
call the FFI, deserialize the requests, re-read the view on `Render`, dispatch the
rest, resolve and repeat — and it handles `Render` itself, so your handler no
longer implements `render` at all. See
[the generated Core](../part-4/typegen.md#the-generated-core) for the shapes.

With it, a shell writes two things:

1. **A bridge adapter.** A type conforming to `CoreBridge` whose three methods
   call the `update`, `resolve` and `view` that BoltFFI generated for your
   crate. Three lines of body in Swift (converting `Data` to `[UInt8]`), and
   pass-through in Kotlin and TypeScript.
2. **An effect handler.** The `EffectHandler` methods that used to live on your
   hand-written core object, and whatever state they need, on a plain class of
   their own. Delete `render`.

Then delete the hand-written loop and construct `Core(bridge, handler, ...)`,
with a callback that publishes the view in Swift, TypeScript and C#, or by
collecting `core.view` in Kotlin. The per-language sections below show the
result for the notes and weather examples.

### TypeScript

Implement the handler on the class that already owned the effect loop, and let
the dispatcher replace the nested `switch`. From the notes example:

```typescript
// Before — nested match helpers, hand-built responses, and an id in a ref
private processEffect(id: number, effect: Effect) {
  matchEffect(effect, {
    Render: () => this.setState(this.view()),
    PubSub: ({ value: op }) => matchPubSubOperation(op, {
      Publish: (op) => this.channel.current.postMessage({ kind: "change", data: op.value }),
      Subscribe: () => { this.subscriptionId.current = id; },
    }),
    KeyValue: ({ value: op }) => matchKeyValueOperation(op, {
      Get: (op) => {
        const result = keyValueResultOk(keyValueResponseGet(value));
        this.respond(id, (s) => serializeKeyValueResult(result, s));
      },
      Delete: () => unsupported("KeyValue::Delete"),
      // … an arm per variant the app never issues
    }),
    // … Time, likewise
  });
}
```

```typescript
// After — a bridge over the FFI, a handler with one method per operation, and
// the generated Core owning the loop between them
export class LiveBridge implements CoreBridge {
  private readonly ffi = CoreFfi.new();
  update(event: Uint8Array): Uint8Array { return this.ffi.update(event); }
  resolve(id: number, output: Uint8Array): Uint8Array { return this.ffi.resolve(id, output); }
  view(): Uint8Array { return this.ffi.view(); }
}

export class NotesHandler implements EffectHandler {
  constructor(/* the refs the handlers need */) {}

  publish(operation: Publish): void {
    this.channel.current.postMessage({ kind: "change", data: operation.value });
  }

  subscribe(_operation: Subscribe, sink: EffectSink<Message>): void {
    this.subscription.current = sink;
  }

  kvGet(operation: Get): Promise<ValueResult> {
    const data = window.localStorage.getItem(operation.key);
    const bytes: number[] = data == null ? [] : JSON.parse(data);
    return Promise.resolve(valueResultOk(bytes.length === 0 ? valueNone() : valueBytes(bytes)));
  }
}

// once the WASM module has initialised:
const core = new Core(new LiveBridge(), new NotesHandler(/* … */), setView);
core.update(eventOpen());
```

`matchEffect`, the `unsupported()` helper for operations the app never issues,
the hand-built response constructors, the stashed request id, `render`, the
hand-rolled `deserializeRequests` and the resolve-and-recurse callback all go.
The stream is the biggest win: instead of remembering a `subscriptionId` and
resolving it repeatedly by hand, the shell parks the `EffectSink` and calls
`sink.send(new Message(bytes))` per message. Note that `CoreFfi.new()` needs the
WASM module initialised, and `Core`'s constructor reads the view straight away,
so construct both after awaiting `initialized` — not in a `useRef` initialiser.

### Swift

```swift
// Before — a switch, and a resolve call per capability
func processEffect(_ request: Request) {
    switch request.effect {
    case .render:
        view = bridge.currentView()
    case let .secret(secretRequest):
        resolveSecret(request: secretRequest, requestId: request.id)
    case let .http(httpRequest):
        resolveHttp(request: httpRequest, requestId: request.id)
    // …
    }
}
```

```swift
// After — a bridge over the FFI, a handler with one method per operation, and
// the generated Core owning the loop between them
nonisolated struct LiveBridge: CoreBridge, @unchecked Sendable {
    private let ffi = CoreFfi()
    func update(_ event: [UInt8]) -> [UInt8] { [UInt8](ffi.update(data: Data(event))) }
    func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8] { [UInt8](ffi.resolve(id: id, data: Data(output))) }
    func view() -> [UInt8] { [UInt8](ffi.view()) }
}

@MainActor public final class WeatherHandler {
    let keyValueStore: KeyValueStore
    var activeTimers: [UInt64: Timer] = [:]
}

nonisolated extension WeatherHandler: EffectHandler {
    public func http(_ operation: HttpRequest) async -> HttpResult {
        await performHttpRequest(operation)
    }

    public func fetchSecret(_ operation: Fetch) async -> SecretFetchResponse {
        keychainGet(key: operation.value).map { .fetched($0) } ?? .missing(operation.value)
    }
}

// in the app:
let store = ViewStore()
let core = Core(bridge: LiveBridge(), handler: WeatherHandler()) { store.view = $0 }
core.update(.start)
```

Each per-capability `switch` over an operation enum collapses into one method per
operation, every `resolve(requestId:serialize:)` call disappears, and so do the
loop, the resolve-and-recurse callback and `render` — the generated `Core`
intercepts `Render` and hands the new view to the closure you give it. `Core`
carries the same `@available` as the dispatcher, so it can't be `@Observable`;
keep a small `@Observable` holder (`ViewStore` above) for SwiftUI to read.

### Kotlin

A bridge over the FFI, a handler that delegates, and the generated `Core`
provided by Hilt:

```kotlin
@Singleton
class LiveBridge @Inject constructor() : CoreBridge {
    private val coreFfi = CoreFfi()
    override fun update(event: ByteArray): ByteArray = coreFfi.update(event)
    override fun resolve(id: UInt, output: ByteArray): ByteArray = coreFfi.resolve(id, output)
    override fun view(): ByteArray = coreFfi.view()
}

@Singleton
class WeatherHandler @Inject constructor(/* … */) : EffectHandler {
    override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)
    override suspend fun kvGet(operation: Get): ValueResult = keyValueHandler.get(operation)
    override fun timeClear(operation: Clear) = timeHandler.clear(operation)
}

@Module @InstallIn(SingletonComponent::class)
object CoreModule {
    @Provides @Singleton fun provideCoreBridge(bridge: LiveBridge): CoreBridge = bridge

    @Provides @Singleton
    fun provideCore(bridge: CoreBridge, handler: WeatherHandler): Core =
        Core(bridge, handler, CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate))
            .also { it.update(Event.Start) }
}
```

The generated `Core` has no `@Inject` constructor, hence the module; it
dispatches each request in its own coroutine on the scope you give it, publishes
the view on `core.view: StateFlow<ViewModel>`, and needs no `render` override.
Because `Core` uses `StateFlow` and `launch`, the Gradle module that compiles the
generated sources needs `kotlinx-coroutines-core` on its classpath — the
generated `build.gradle.kts` declares it, but if you pull the sources in with
`srcDirs`, add it yourself.

The injected handlers lose their `when` blocks too: `KeyValueHandler.get` takes a
`Get` and returns a `ValueResult`, rather than matching a wide operation enum and
constructing the matching response variant.

### C#

The shapes are the same with .NET naming: `IEffectHandler` with `Task<T>` request
methods, `IEffectSink<in T>` for streams, and
`new EffectDispatcher(handler, resolve)` whose `Dispatch(request)` you call per
request.

### Keeping a flat match

Rust shells should not use the generated handler API — there is nothing to
generate. A `match` over the `Effect` enum is already exactly as precise,
because each variant carries its operation type and the compiler knows what
output that request resolves with. The weather Leptos shell just grew from six
arms to eleven:

```rust,ignore
fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
    match effect {
        Effect::Render(_) => render.set(core.view()),
        Effect::Http(request) => http::resolve(core, request, render),
        Effect::KvGet(request) => kv::get(core, request, render),
        Effect::KvSet(request) => kv::set(core, request, render),
        Effect::TimeNotifyAfter(request) => time::notify_after(core, request, render),
        Effect::TimeClear(request) => time::clear(request.operation),
        Effect::IsLocationEnabled(request) => location::is_location_enabled(core, request, render),
        // …
    }
}
```

The same applies to a non-Rust shell that wants full control over its own
concurrency: the emission is additive, and ignoring it costs nothing.

---

## Traps worth knowing about

These are the things that actually caught us out migrating the two examples.

### Never resolve a cleared timer

`operation::Clear` is a notification, so by the time the shell sees it the core
has already stopped waiting for the timer. Resolving the original
`NotifyAfter` request afterwards is a `NotFound`, which the FFI surfaces as a
panic. Each shell has to drop or cancel the pending timer rather than let it
fire:

- Swift invalidates the `Timer`, which releases the continuation
  `timeNotifyAfter` is suspended on;
- Leptos keeps a `HashMap<usize, Timeout>` and removes the entry, since dropping
  a `gloo_timers::Timeout` cancels it;
- TypeScript calls `window.clearTimeout` on the stored handle;
- Kotlin cancels the coroutine running the delay.

Under `Time`, by contrast, the shell answered a `Clear` with
`TimeResponse::Cleared`. Delete that response.

### Swift: `EffectHandler` and `CoreBridge` are `Sendable`, your handler is probably `@MainActor`

The generated operation and output types are not `Sendable`, so a `@MainActor`
class cannot witness the `Sendable` protocol's non-isolated requirements. The
pattern that works is a `nonisolated` extension that hops to the main actor only
where it touches main-actor state:

```swift
nonisolated extension WeatherHandler: EffectHandler {
    public func kvGet(_ operation: Get) async -> ValueResult {
        await MainActor.run { keyValueStore.get(operation.key) }
    }
}
```

URLSession, Keychain and CoreLocation work does not belong on the main actor
anyway, so this is usually an improvement. Only `Sendable` values cross back.

The bridge has the same shape of problem from the other side: `CoreBridge` is
`Sendable`, but BoltFFI's `CoreFfi` is a class Swift can't prove safe. Declare
the adapter `nonisolated` (if your target defaults to `MainActor` isolation)
and `@unchecked Sendable` — sound, because the Rust `Bridge` behind the handle
guards its state with mutexes.

### `Set` collides with the standard library

`crux_kv`'s `Set` operation generates a type called `Set` in every language, and
Swift, Kotlin and TypeScript all have one already. Alias it at the import:

```kotlin
import com.example.weather.Set as KeyValueSet
```

```typescript
import type { Set as SetValue } from "shared_types/app";
```

### `Clock`'s cleared outcome no longer waits for an ack

`Time::notify_after`'s future resolved `TimerOutcome::Cleared` only once the
shell had acknowledged the clear. `Clock`'s resolves immediately. If any of your
core logic relied on the round trip to sequence something after a clear, it now
happens sooner.

---

## Deprecations

Everything below still works in this release and warns with its replacement
named. All of it is removed in the next breaking release.

| Item | Since | Use instead |
| --- | --- | --- |
| `crux_kv::KeyValue` | `crux_kv` 0.15.0 | `crux_kv::KeyValueStore` |
| `crux_kv::KeyValueOperation` | `crux_kv` 0.15.0 | `crux_kv::operation::{Get, Set, Delete, Exists, ListKeys}` |
| `crux_kv::KeyValueResult` | `crux_kv` 0.15.0 | `crux_kv::operation::{ValueResult, BoolResult, KeysResult}` |
| `crux_kv::KeyValueResponse` | `crux_kv` 0.15.0 | the output type of the operation you sent |
| `crux_time::Time` | `crux_time` 0.19.0 | `crux_time::Clock` |
| `crux_time::TimeRequest` | `crux_time` 0.19.0 | `crux_time::operation::{Now, NotifyAt, NotifyAfter, Clear}` |
| `crux_time::TimeResponse` | `crux_time` 0.19.0 | `Instant`, `TimerId`, or nothing for `Clear` |
| `crux_time::TimerFuture` | `crux_time` 0.19.0 | nothing — an implementation detail of `Time` |

Not deprecated, and shared by both APIs: `crux_kv::{KeyValueError, Value,
DataResult, StatusResult, ListResult}` and `crux_time::{TimerHandle,
CompletedTimerHandle, TimerOutcome, TimerId, Instant, Duration}`.

If you need to keep using the old API for now, `#[allow(deprecated)]` on the
module or the item silences the warning.

---

## Coming in the breaking release

Written against this release's derive and marker traits, your code does not
change. What changes:

- **The kind becomes an associated type.** `type Kind: operation::Kind` replaces
  `const KIND`, with sealed unit types `operation::kind::{Notify, Request,
  Stream}`, and the markers `operation::{Notify, Request, Stream}` become blanket
  impls from it. The derive emits `type Kind = kind::Request;` instead of a const
  plus a marker impl, so only hand-written `impl Operation` blocks need editing.
- **`Command` bounds tighten** to the markers, so the wrong constructor is an
  ordinary `E0277` you see in `cargo check` and in your editor — with a
  `#[diagnostic::on_unimplemented]` message naming the right one — and the
  post-monomorphisation `const` assertion goes.
- **The deprecated items above are removed**, along with the `command` module
  re-export shims and the legacy "no declared kind" handling in the bridge and in
  type generation. Every operation will have to declare a kind.
- **The remaining examples migrate** — `counter`, `counter-http`,
  `counter-middleware` and `counter-routing`, and their shells.

This guide will be extended when that release lands.
