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
4. **Regenerate your shells** and either adopt the generated `EffectHandler`, or
   widen the match you already have.
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

### `register(..)` for type generation

`#[operation(request, output = T, register(A, B))]` names extra types for type
generation to emit — the ones its tracer cannot reach through the output alone.
It generates `register_types` / `register_types_facet` overrides gated on the
`typegen` and `facet_typegen` features **of your crate**. A crate that declares
neither still compiles (the generated `impl` carries
`#[allow(unexpected_cfgs)]`) but gets no overrides, so if you rely on
`register(..)`, forward the features:

```toml
[features]
typegen = ["crux_core/typegen"]
facet_typegen = ["crux_core/facet_typegen"]
```

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
// After — one method per operation, returning its output
export class Core implements EffectHandler {
  private readonly dispatcher: EffectDispatcher;

  constructor(/* … */) {
    this.dispatcher = new EffectDispatcher(this, (id, bytes) => this.respond(id, bytes));
  }

  render(): void {
    this.setState(this.view());
  }

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
```

`matchEffect`, the `unsupported()` helper for operations the app never issues,
the hand-built response constructors and the stashed request id all go. The
stream is the biggest win: instead of remembering a `subscriptionId` and
resolving it repeatedly by hand, the shell parks the `EffectSink` and calls
`sink.send(new Message(bytes))` per message.

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
// After — the dispatcher does the switching and the resolving
dispatcher = EffectDispatcher(handler: self) { [weak self] requestId, responseBytes in
    Task { @MainActor [weak self] in
        guard let self else { return }
        self.process(self.bridge.resolve(requestId: requestId, responseBytes: responseBytes))
    }
}

private func process(_ requests: [Request]) {
    for request in requests { dispatcher.dispatch(request) }
}

nonisolated extension Core: EffectHandler {
    public func render(_: RenderOperation) {
        Task { @MainActor in refreshView() }
    }

    public func http(_ operation: HttpRequest) async -> HttpResult {
        await performHttpRequest(operation)
    }

    public func fetchSecret(_ operation: Fetch) async -> SecretFetchResponse {
        keychainGet(key: operation.value).map { .fetched($0) } ?? .missing(operation.value)
    }
}
```

Each per-capability `switch` over an operation enum collapses into one method per
operation, and every `resolve(requestId:serialize:)` call disappears.

### Kotlin

`Core` implements the interface and delegates:

```kotlin
class Core @Inject constructor(/* … */) : EffectHandler {
    private val dispatcher = EffectDispatcher(this) { requestId, data ->
        scope.launch { resolveAndHandleEffects(requestId, data) }
    }

    private fun processRequest(request: Request) {
        // one coroutine per request: `dispatch` suspends for as long as the
        // handler does, and a timer must not hold up what's queued behind it
        scope.launch { dispatcher.dispatch(request) }
    }

    override fun render(operation: RenderOperation) = render()
    override suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)
    override suspend fun kvGet(operation: Get): ValueResult = keyValueHandler.get(operation)
    override fun timeClear(operation: Clear) = timeHandler.clear(operation)
}
```

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

### Swift: `EffectHandler` is `Sendable`, your `Core` is probably `@MainActor`

The generated operation and output types are not `Sendable`, so a `@MainActor`
class cannot witness the `Sendable` protocol's non-isolated requirements. The
pattern that works is a `nonisolated` extension that hops to the main actor only
where it touches main-actor state:

```swift
nonisolated extension Core: EffectHandler {
    public func render(_: RenderOperation) {
        Task { @MainActor in refreshView() }
    }
}
```

URLSession, Keychain and CoreLocation work does not belong on the main actor
anyway, so this is usually an improvement. Only `Sendable` values cross back.

### `Set` collides with the standard library

`crux_kv`'s `Set` operation generates a type called `Set` in every language, and
Swift, Kotlin and TypeScript all have one already. Alias it at the import:

```kotlin
import com.example.weather.Set as KeyValueSet
```

```typescript
import type { Set as SetValue } from "shared_types/app";
```

### `register(..)` and `unexpected_cfgs`

Covered [above](#register-for-type-generation): if your crate does not declare
`typegen` / `facet_typegen` features, `register(..)` compiles but does nothing.
Forward the features if you need it.

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
