# RFC: Per-operation types with static request kinds

```admonish
This RFC has been **accepted**. The compat stage shipped in `crux_core` 0.21 —
see the [migration guide](../guide/migrate-per-operation-types.md) — and the
breaking stage is scheduled for the next major release. The text below is kept
as it was written, with the sections describing what shipped brought up to
date.
```

This RFC proposes that each operation a capability can ask the shell to perform
becomes its own type, carrying exactly one output type and exactly one request
kind (notify, request or stream), declared statically on the type rather than
chosen at the call site.

## Summary

Today a capability implements `Operation` on a coarse enum, and every variant of
that enum shares one `Output` type and can be sent with any of the three
`Command` constructors. Two consequences follow, and Crux users run into both:

1. A response can be a well-formed value of the wrong variant. Every capability
   has to match on the response and decide what to do when it does not
   correspond to the operation that was sent. `crux_kv` and `crux_time` both
   panic in that case.
2. Nothing ties an operation to the number of times it will be resolved. The
   same variant can be notified in one place and requested in another, so the
   kind cannot be typegen'd for shells, and a shell that resolves the wrong
   number of times either crashes the core or hangs a command forever.

The proposal is to move both facts into the type system:

```rust
// One type per operation. The kind and the output are properties of the type.
#[derive(Operation)]
#[operation(request, output = ValueResult)]
pub struct Get { pub key: String }

#[derive(Operation)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

#[derive(Operation)]
#[operation(stream, output = Message)]
pub struct Subscribe;

// Or by hand, in the target shape:
impl Operation for Get {
    type Output = ValueResult;
    type Kind = operation::kind::Request;
}
```

With the kind on the type:

- `Command::notify_shell` accepts only `operation::Notify`, `request_from_shell`
  only `operation::Request`, and `stream_from_shell` only `operation::Stream`.
  Sending an operation with the wrong kind stops compiling.
- The core deserializes a response into the specific `Output` for that
  operation. A wrong response fails at the boundary instead of arriving as a
  valid value of another variant. The `unwrap_get` family of helpers and the
  `WrongResponse` variants go away.
- The kind needs no bytes on the wire. It is a static property of each
  `Effect` variant, so type generation can emit it, and can generate a
  shell-side handler API where a request handler returns exactly one value, a
  stream handler is handed a sink, and a notify handler has nothing to resolve.

## Why?

### Operation and Output are too coarse

`Operation` already says the right thing: one operation type has one output
type.

```rust
pub trait Operation: Send + 'static {
    type Output: Send + Unpin + 'static;
}
```

The problem is that by convention the trait is implemented on an enum. Here is
`crux_kv`:

```rust
pub enum KeyValueOperation {
    Get { key: String },
    Set { key: String, value: Vec<u8> },
    Delete { key: String },
    Exists { key: String },
    ListKeys { prefix: String, cursor: u64 },
}

pub enum KeyValueResponse {
    Get { value: Value },
    Set { previous: Value },
    Delete { previous: Value },
    Exists { is_present: bool },
    ListKeys { keys: Vec<String>, next_cursor: u64 },
}

impl Operation for KeyValueOperation {
    type Output = KeyValueResult; // Ok { response: KeyValueResponse } | Err { .. }
}
```

A `KeyValueResponse::Set` is a perfectly valid response to a
`KeyValueOperation::Get`, as far as the type system and the deserializer are
concerned. So the capability has to check:

```rust
pub fn unwrap_get(self) -> Result<Option<Vec<u8>>, KeyValueError> {
    match self {
        Self::Ok { response } => match response {
            KeyValueResponse::Get { value } => Ok(value.into()),
            _ => panic!("attempt to convert KeyValueResponse other than Get to Option<Vec<u8>>"),
        },
        Self::Err { error } => Err(error),
    }
}
```

`crux_time` does the same:

```rust
let TimeResponse::Cleared { id } = ctx.request_from_shell(TimeRequest::Clear { id: cleared_id }).await else {
    panic!("Unexpected response to TimeRequest::Clear");
};
```

Every capability author writes this check, and every capability author has to
decide between panicking, inventing an error variant, or silently ignoring the
response. None of those is a good answer to a bug that the type system could
have ruled out. The response enum exists only because the operation enum does.
Each operation already knows what it returns; the enum is what forgets.

### The request kind lives at the call site

The three `Command` constructors accept any operation:

```rust
pub fn notify_shell<Op: Operation>(operation: Op) -> NotificationBuilder<..>
pub fn request_from_shell<Op: Operation>(operation: Op) -> RequestBuilder<.., Op::Output>
pub fn stream_from_shell<Op: Operation>(operation: Op) -> StreamBuilder<.., Op::Output>
```

So the kind is a property of *which constructor was called*, not of the
operation. The core records it in the `RequestHandle` it builds, and until
recently that was the only place it existed.

Different variants do genuinely want different kinds. The notes example's
pub/sub capability is the clearest case:

```rust
pub enum PubSubOperation {
    Publish(Vec<u8>),
    Subscribe,
}

impl Operation for PubSubOperation {
    type Output = Message;
}

Command::notify_shell(PubSubOperation::Publish(data))
Command::stream_from_shell(PubSubOperation::Subscribe).map(|Message(data)| data)
```

`Publish` is a notification with no meaningful output. `Subscribe` is a stream
of `Message`. They share an `Output` type that only one of them uses, and there
is nothing stopping someone calling `request_from_shell(PubSubOperation::Publish(..))`,
which would hang forever waiting for a response the shell will never send.

Note what this example does *not* show: the same variant used with two
different kinds. We have not found a case of that anywhere in the Crux
repository or its examples. The kind varies per variant, which is exactly what
per-variant types can express and per-enum types cannot.

[PR #580](https://github.com/redbadger/crux/pull/580), open at the time of
writing, makes the kind visible on the resolve path by encoding it in two bits
of the request id. It is a cheap, non-breaking way to surface the kind and it
introduces the `RequestKind` type this RFC builds on. But it records the kind
*per request instance*, which is more freedom than anyone uses, and it puts the
information somewhere shells can only reach by unpacking an id whose encoding
is documented as an implementation detail.

Once the kind is a static property of each operation type, the id no longer
needs to carry it. The core knows the operation's static kind when it registers a
request and when it resolves one, and shells know it from the generated
per-variant table. This RFC therefore **supersedes the id encoding**: the
`RequestKind` type and the `kind()` accessors on handles, resolvers and parked
requests from that PR stay, and the two id bits go. If the PR merges first, the
bits are removed in the compat release below; if this RFC is accepted first, the PR can
drop the encoding before it lands.

### Shells inherit both problems

A shell receives `Request { id, effect }` and has to know, for each effect
variant, which response variant to construct and how many times to resolve. It
learns that by reading capability source or documentation. Getting either wrong
is a runtime failure, and the "never resolves" failure is silent: the core keeps
processing other work while one command waits forever.

Both facts are statically known in Rust. They should be statically known in
Swift, Kotlin and TypeScript too.

## Goals

1. An operation type has exactly one output type and exactly one request kind.
2. Sending an operation with the wrong `Command` constructor is a compile error.
3. Resolving an operation with a value of the wrong type is a compile error in
   the typed lanes (effect router, middleware, Rust shells) and a deserialization
   error at the boundary in the serialized lane.
4. The kind reaches shells through type generation with no change to the wire
   format.
5. Type generation can emit a per-effect handler API in which the number of
   resolves is fixed by the signature.
6. Capability authors write less boilerplate than today, not more.

## Non-goals

- Changing the wire format. `Request { id, effect }` stays as it is.
- Redesigning `Command`, the effect router or middleware. They gain tighter
  bounds and lose some runtime checks, nothing more.
- Solving stream termination. A way for the core to tell the shell that a
  stream has finished is worth having and is made easier by knowing the kind,
  but it is a separate RFC.
- Making bincode self-describing. Two structs with the same layout still
  deserialize into each other. This RFC removes the *designed-in* ability to
  return the wrong variant; it does not detect arbitrary byte-level mistakes.

## Design

### The kind is an associated type

The target shape puts the kind on `Operation` as an associated type, so that
the wrong constructor fails with an ordinary trait-bound error:

```rust
/// How many times a request expects to be resolved. Introduced by PR #580.
pub enum RequestKind { Notify, Request, Stream }

pub mod operation {
    /// Common base: a serializable payload the shell can act on. Carries the
    /// typegen registration hooks that `Operation` carries today.
    pub trait Operation: Send + 'static {
        /// The value the shell resolves with. `()` for notifications.
        type Output: Send + Unpin + 'static;

        /// How many times this operation expects to be resolved.
        type Kind: Kind;

        // register_types / register_types_facet as today
    }

    /// One of the three kinds, as a type. Sealed; the only impls are below.
    pub trait Kind: sealed::Sealed {
        const VALUE: RequestKind;
    }

    pub mod kind {
        pub struct Notify;   // impl Kind { VALUE = RequestKind::Notify }
        pub struct Request;  // impl Kind { VALUE = RequestKind::Request }
        pub struct Stream;   // impl Kind { VALUE = RequestKind::Stream }
    }

    /// Fire and forget. Nothing waits on it.
    pub trait Notify: Operation<Output = (), Kind = kind::Notify> {}
    impl<Op: Operation<Output = (), Kind = kind::Notify>> Notify for Op {}

    /// Exactly one response.
    pub trait Request: Operation<Kind = kind::Request> {}
    impl<Op: Operation<Kind = kind::Request>> Request for Op {}

    /// Zero or more responses.
    pub trait Stream: Operation<Kind = kind::Stream> {}
    impl<Op: Operation<Kind = kind::Stream>> Stream for Op {}
}
```

`Operation` keeps `Output` and gains `Kind`, so `Request<Op>`, the bridge
registry, the effect router and middleware keep working against one trait and
can read the kind statically as `<Op::Kind as Kind>::VALUE`. The three marker
traits, `operation::{Notify, Request, Stream}`, exist so that bounds can name a
kind. They are blanket-implemented from `Kind`, so an author declares the kind
exactly once and nothing can disagree with it. The names deliberately shadow
`crux_core::Request<Op>` and `futures::Stream`; import the module and write
`Op: operation::Request`, not the items.

`operation::Stream` uses `Output` for the item type. `type Item` would read
better, but it would need a second associated type on `Operation` or a way to
express "Output is the item" that generic code can use uniformly.

**This shape is breaking.** Associated type defaults are unstable (E0658,
rust-lang/rust#29661), so `type Kind` cannot default to "unspecified" and every
existing `impl Operation` would have to declare one. The compat release
therefore ships a transitional shape, described under Migration, and the
breaking release switches to the one above. The public bounds
`Op: operation::Notify | Request | Stream` are the same in both, so code written
against the compat release does not change.

### The Command constructors take the marker traits

```rust
impl<Effect, Event> Command<Effect, Event> {
    pub fn notify_shell<Op>(operation: Op) -> NotificationBuilder<Effect, Event, impl Future<Output = ()>>
    where
        Op: operation::Notify,
        Effect: From<Request<Op>>;

    pub fn request_from_shell<Op>(operation: Op) -> RequestBuilder<Effect, Event, impl Future<Output = Op::Output>>
    where
        Op: operation::Request,
        Effect: From<Request<Op>>;

    pub fn stream_from_shell<Op>(operation: Op) -> StreamBuilder<Effect, Event, impl Stream<Item = Op::Output>>
    where
        Op: operation::Stream,
        Effect: From<Request<Op>>;
}
```

The same bounds apply to the `CommandContext` methods used inside `async`
blocks. Nothing else about `Command` changes. Passing a notification to
`request_from_shell` then reads, in `cargo check` and in the editor:

```text
error[E0277]: the trait bound `Publish: operation::Request` is not satisfied
  = help: the following other types implement trait `operation::Request`: ...
```

A `#[diagnostic::on_unimplemented]` attribute on each marker turns that into
"`Publish` is a notification; send it with `notify_shell`".

### Declaring operations

The trait shape above is verbose to implement by hand for a capability with
five operations, so a derive does it:

```rust
#[derive(Facet, Serialize, Deserialize, Operation)]
#[operation(request, output = ValueResult)]
pub struct Get {
    pub key: String,
}

#[derive(Facet, Serialize, Deserialize, Operation)]
#[operation(notify)]
pub struct Publish(pub Vec<u8>);

#[derive(Facet, Serialize, Deserialize, Operation)]
#[operation(stream, output = Message)]
pub struct Subscribe;
```

The derive generates the `Operation` impl with `Output` and `Kind`, and the
typegen registration that today's impls write by hand. The marker follows from
`Kind` by the blanket impls, so there is nothing else to write.

Outputs are concrete types that type generation can emit. `ValueResult` here is
an `Ok(Value) | Err(KeyValueError)` enum in the style of `HttpResult`, not
`Result<Option<Vec<u8>>, KeyValueError>`: the generators have no emission for
`std::result::Result`, and two `Result`s with different parameters would collide
in one registry. `From` impls convert the wire enum to the developer-facing
`Result` alias the capability's builders return today.

For a capability with many small operations, the derive can also be applied to
an enum and split it, so that authors who prefer to see their operations in one
place can:

```rust
#[derive(Operation)]
pub enum KeyValue {
    #[operation(request, output = ValueResult)]
    Get { key: String },
    #[operation(request, output = ValueResult)]
    Set { key: String, value: Vec<u8> },
    // ...
}
```

This would generate one struct per variant, in a module named after the enum.
Whether this second form is worth the macro complexity is an open question. The
first form is the proposal; the second is sugar. *It was not built: the derive
that shipped is struct-only, and one struct per operation reads well enough
that nobody has asked for the sugar.*

### The Effect enum

The `#[effect]` macro already generates one variant per operation type, so the
only change is that there are more of them:

```rust
#[effect(facet_typegen)]
pub enum Effect {
    Render(RenderOperation),
    Http(HttpRequest),
    KvGet(kv::Get),
    KvSet(kv::Set),
    KvDelete(kv::Delete),
    KvExists(kv::Exists),
    KvListKeys(kv::ListKeys),
    Publish(pub_sub::Publish),
    Subscribe(pub_sub::Subscribe),
}
```

This is the most visible cost of the proposal and is discussed under drawbacks.
Each variant now carries an operation whose `Kind` and `Output` are known at
compile time, which is what the rest of the design relies on.

`crux_http` is already in the target shape: one `HttpRequest` type with one
`HttpResult` output, always requested. Under this RFC it gains
`type Kind = operation::kind::Request;` and nothing else changes.

### The serialized lane and the wire

`Request { id: EffectId, effect: EffectFfi }` does not change. The `EffectFfi`
enum has one variant per operation, as it does today, and each variant's kind
is a compile-time constant on the Rust side.

The bridge registry can therefore read the operation's static kind when it
registers a request rather than inspecting the `ResolveSerialized` it was
handed. The id goes back
to being a plain sequence number: with the kind known statically on both sides
of the boundary, encoding it into the id as well would duplicate information
the shell already has from the generated table, and would keep an encoding
alive that shells are told not to depend on. The one thing the id encoding
could do that a static kind cannot is distinguish a resolve of a notification
from a resolve of an unknown id without storing an entry for every
notification. The registry can keep that distinction by recording the kind of
the most recently issued ids in a small ring rather than in the id itself, or
by accepting that both cases report `NotFound`, which is what 0.20 does today.

The `Output` types the bridge deserializes into become specific to the
operation. A response that does not parse as the expected `Output` is reported
as `BridgeError::DeserializeOutput`, which is a clearer failure than a valid
value of the wrong variant that later panics inside a capability.

### Type generation

*This section describes what shipped in 0.21, which is more than the RFC
originally proposed: the handler API was a "second phase" here and landed in the
same release.*

Because the kind is static per `EffectFfi` variant, type generation emits it as
a property of the generated effect type, with no wire cost. It also emits a
handler protocol/interface and a dispatcher, so a shell can hand each effect to
a method whose signature already says how many times it will be resolved and
with what.

Everything below is emitted next to the generated `Effect`, in Swift, Kotlin,
TypeScript and C#, by plugins that live in `crux_core`
(`type_generation::facet::plugins`) rather than in facet-generate. The names
`RequestKind`, `EffectSink`, `EffectHandler` (`IEffectSink` / `IEffectHandler`
in C#) and `EffectDispatcher` are reserved: `TypeRegistry::build` reports an
error if a shared type or an effect variant claims one.
`CodeGenerator::without_effect_handlers()` turns the handler half off.

Taking an effect with one variant of each kind, plus one legacy operation that
declares no kind:

Swift:

```swift
public enum RequestKind: Hashable, Sendable {
    case notify, request, stream
}

extension Effect {
    public var requestKind: RequestKind? { /* .render -> .notify, ... */ }
}

public struct EffectSink<Item>: Sendable {
    public func send(_ item: Item)
}

@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)
public protocol EffectHandler: Sendable {
    func render(_ operation: RenderOperation)
    func http(_ operation: HttpRequest) async -> HttpResult
    func subscribe(_ operation: Subscribe, into sink: EffectSink<Message>)
    func legacy(_ operation: LegacyOperation, requestId: UInt32,
                resolve: @escaping @Sendable ([UInt8]) -> Void)
}

@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)
public struct EffectDispatcher: Sendable {
    public init(handler: any EffectHandler,
                resolve: @escaping @Sendable (UInt32, [UInt8]) -> Void)
    public func dispatch(_ request: Request)
}
```

Kotlin:

```kotlin
enum class RequestKind { NOTIFY, REQUEST, STREAM }

val Effect.requestKind: RequestKind?

fun interface EffectSink<in T> { fun send(item: T) }

interface EffectHandler {
    fun render(operation: RenderOperation)
    suspend fun http(operation: HttpRequest): HttpResult
    fun subscribe(operation: Subscribe, sink: EffectSink<Message>)
    fun legacy(operation: LegacyOperation, requestId: UInt, resolve: (ByteArray) -> Unit)
}

class EffectDispatcher(handler: EffectHandler, resolve: (UInt, ByteArray) -> Unit) {
    suspend fun dispatch(request: Request)
}
```

TypeScript — the union's discriminant is already `kind`, so the accessor is a
free function rather than a property:

```typescript
export type RequestKind = "notify" | "request" | "stream";
export function effectRequestKind(effect: Effect): RequestKind | undefined;

export interface EffectSink<T> { send(item: T): void }

export interface EffectHandler {
    render(operation: RenderOperation): void;
    http(operation: HttpRequest): Promise<HttpResult>;
    subscribe(operation: Subscribe, sink: EffectSink<Message>): void;
    legacy(operation: LegacyOperation, requestId: uint32,
           resolve: (bytes: Uint8Array) => void): void;
}

export class EffectDispatcher {
    constructor(handler: EffectHandler,
                resolve: (id: uint32, bytes: Uint8Array) => void);
    public dispatch(request: Request): void;
}
```

C#:

```csharp
public enum RequestKind { Notify, Request, Stream }

// on the generated Effect record
public RequestKind? RequestKind { get; }

public interface IEffectSink<in T> { void Send(T item); }

public interface IEffectHandler
{
    void Render(RenderOperation operation);
    Task<HttpResult> Http(HttpRequest operation);
    void Subscribe(Subscribe operation, IEffectSink<Message> sink);
    void Legacy(LegacyOperation operation, uint requestId, Action<byte[]> resolve);
}

public sealed class EffectDispatcher
{
    public EffectDispatcher(IEffectHandler handler, Action<uint, byte[]> resolve);
    public void Dispatch(Request request);
}
```

Two language details are worth recording. The Swift protocol and dispatcher
carry `@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)`, because
`Task {}` needs those versions and facet-generate's own `Package.swift`
declares no platforms; a package that declares `platforms:` conforms without
repeating the annotation. In C#, the generated `Effect` record is not
`partial`, so the kind accessor is emitted inside the record body rather than
as an extension, and the `RequestKind` enum is namespace-qualified where it is
used.

The dispatcher resolves each request for the shell: never for a notification,
once for a request with whatever the handler method returned, and once per item
a stream's sink receives. Outputs are serialized with the generated bincode
serializers, so there is no `resolve` call left for a shell to get wrong. An
operation that declares no kind — everything that has not migrated — keeps the
shape it always had: its handler method is handed the request id and a
`resolve` callback taking raw bytes. Shells that prefer to match on `Effect`
and resolve by hand are unaffected; the emission is purely additive.

### Effect router and middleware

`Parked<Op>` already knows its `Op`, so it knows the kind statically.
`EffectResolver<Output>` is generic over the output rather than the operation,
so it keeps the runtime `kind()` accessor added by PR #580, as do
`RequestHandle` and `ResolveSerialized`; those remain the runtime view of the
same fact. Routes and middleware written against a specific `Op` can be bounded
on `operation::Request` or `operation::Stream` where it matters, for example a
route that only makes sense for streams.

## Drawbacks

**The `Effect` enum gets wider.** An app using `crux_kv` goes from one variant
to five. The shell's match statement widens to match. Each arm is precisely
typed, and the generated handler API removes the match altogether for shells
that adopt it, but the raw enum is bigger. A nested form, where the effect
macro accepts a capability-level enum of operations and flattens it for the FFI,
would keep the app-level enum short; it is listed as an open question because it
adds macro complexity and a second way to do things.

**More types per capability.** Five structs instead of one enum and one response
enum. The derive keeps the declaration to roughly the same number of lines as
today, and the response enum and its `unwrap_*` helpers disappear, so the net
line count for `crux_kv` is likely lower. It is still more names.

**It is a breaking change for every capability and every shell.** Operation
types change shape, so generated shell code changes shape and shells must be
regenerated and their effect handling revisited. Crux has made changes of this
size before, with the Command API and the effect router. Migration is discussed
below.

**Bincode is still not self-describing.** Two operations with structurally
identical outputs cannot be told apart from bytes alone. This RFC removes the
ability to return a valid *wrong variant*, which is the failure people actually
hit. It does not turn the bridge into a schema validator.

**Notifications have `Output = ()`.** This is slightly odd for a type that has
no output at all, but it lets `Request<Op>` and the registries stay generic over
one trait. An alternative without `Output` on the base trait is sketched under
open questions.

## Migration

The change lands in two releases so that each is usable on its own.

**Compat release (additive) — shipped in `crux_core` 0.21, `crux_macros` 0.11,
`crux_http` 0.21, `crux_kv` 0.15 and `crux_time` 0.19.** Everything a reader
needs in order to try the design, without breaking anyone. The
[migration guide](../guide/migrate-per-operation-types.md) is the practical
version of this list:

- `RequestKind` (from PR #580, minus its id-bit encoding) and the `kind()`
  accessors on handles and resolvers.
- The marker traits `operation::{Notify, Request, Stream}`.
- A transitional kind declaration: `const KIND: Option<RequestKind> = None;` on
  `Operation`. A const can have a default where an associated type cannot, so
  every existing impl keeps compiling with `None`, meaning "kind decided by the
  constructor called, as before".
- `#[derive(Operation)]`, which emits `KIND = Some(..)` and the matching marker
  impl together so they cannot disagree.
- `Command` and `CommandContext` keep `Op: Operation` bounds, and check the
  declaration with a `const { assert!(..) }` block. Stable Rust has no way to
  express "declares `Request`, or declares nothing" as a trait bound, so this
  post-monomorphisation assertion is the only compile-time check available
  while legacy impls exist. It fails `cargo build` with an E0080 at the call
  site, but is invisible to `cargo check` and rust-analyzer, and its rendering
  depends on whether `rust-src` is installed. Both warts are accepted as the
  price of not breaking anyone in this release.
- Per-operation rewrites of `crux_kv`, `crux_time` and the example capabilities,
  alongside the existing enum APIs, which are deprecated. `crux_kv::KeyValue`,
  `KeyValueOperation`, `KeyValueResult` and `KeyValueResponse`, and
  `crux_time::Time`, `TimeRequest`, `TimeResponse` and `TimerFuture`, all warn
  and name their replacement.
- Type generation emits the kind for operations that declare one, and the
  handler API and dispatcher, in all four languages.

Two of the examples — `notes` and `weather` — moved across, core and shells,
and the other four stayed on the enum APIs and bare `impl Operation`, which is
the evidence that the two coexist. The one thing the compat release did *not*
ship is the enum-splitting derive (open question 3); one struct per operation
turned out to be fine in practice.

**Breaking release.** Switch to the target shape:

- `type Kind: operation::Kind` replaces `const KIND`, the markers become
  blanket impls from `Kind`, and the derive emits `type Kind = kind::Request;`
  instead of a const and a marker impl.
- The `Command` and `CommandContext` bounds become the marker traits and the
  const assertion goes. The wrong constructor is now an ordinary E0277 in
  `cargo check`, with a `#[diagnostic::on_unimplemented]` message.
- Remove the deprecated enum APIs, the legacy `None` handling in the bridge and
  in type generation, and migrate the remaining examples and their shells.

For users' own capabilities, the mechanical migration is: one struct per
variant, `#[operation(..)]` on each, and replace the response enum with the
per-operation output types. Code written against the compat release's markers
and derive does not change in the breaking release; only hand-written
`impl Operation` blocks swap `const KIND` for `type Kind`.

## Alternatives considered

**Kind per request instance, encoded in the id (PR #580 as it stands).** Zero
wire cost and non-breaking, and a reasonable stopgap if this RFC is not
adopted. But it is runtime information, it cannot drive codegen, and shells can
only read it by unpacking an id whose layout is not a stable contract. It
addresses the resolve path in the core without addressing the two root causes,
and once the kind is static on the type it is redundant.

**Kind as a field on `Request { id, kind, effect }`.** Honest and typegen'd for
free, at four bytes per request under bincode and a regeneration for existing
shells. Still per instance, so still no codegen of a per-variant handler API,
and still one response enum per capability. If the community reports real
cases of one variant used with two kinds, this is the right fallback.

**Keep enum operations, add a `kind()` method to the `Operation` trait that
matches on `self`.** Gets the kind onto the type without splitting it, and could
be checked at runtime in the `Command` constructors. Does nothing for the
output type, and the check is a panic rather than a compile error.

**A second associated type for the response variant.** Keep the enum but add
`type Response<V>` indexed by variant. Rust cannot express associated types per
enum variant, so this needs a proxy type per variant anyway, at which point
per-operation types are simpler.

**Kind as a const with a compile-time assertion, permanently.** This is the
compat release's transitional shape. It could stay: a defaulted const is the
only stable way to let undeclared operations coexist with declared ones, and
the `const { assert!(..) }` does reject the wrong constructor. But the error
only appears on `cargo build`, never in `cargo check` or the editor, and the
const and the marker are two declarations that a hand-written impl can make
disagree. Once the legacy default is gone there is no reason to keep either
wart, so the breaking release moves the kind to an associated type.

## Open questions

Answered by the compat release, in the order they were asked:

1. **`Output` on notifications.** `Output = ()` stayed. Keeping one associated
   type on the base trait is what lets `Request<Op>`, the registries, the
   effect router and middleware all stay generic over `Operation`, and in
   practice nobody notices the unit: `#[operation(notify)]` forbids an `output`
   argument, and the marker `operation::Notify` is bounded on
   `Operation<Output = ()>`, so a hand-written impl that declares something
   else fails to compile.
2. **Nested effect enums.** Not built. The two migrated examples list their
   operations flat — `weather` has eleven variants and `notes` seven — and the
   flat form reads well and gives shells a single exhaustive match, or a single
   handler interface. Listing only the operations an app actually uses turned
   out to matter more than shortening the list: `weather` carries `KvGet` and
   `KvSet` and never has to think about `ListKeys`.
3. **The enum-splitting derive.** Not built, and not missed. One struct per
   operation is roughly the same number of lines as an enum variant plus its
   response variant, and it is what the rest of the design reads.
4. **Resolving a notification.** Both cases report `NotFound`, as 0.20 did. The
   bridge does not store an entry for a notification, so a shell that resolves
   one is told the id is unknown. The generated dispatcher makes this hard to
   do by accident — there is no `resolve` in a notification's handler method —
   and the migration guide calls out the one place it bites: a cleared timer.
5. **Stream termination.** Still separate. The kind reaching the shell makes it
   easier to design, and nothing in the compat release forecloses it.
6. **Error conventions.** `crux_kv` and `crux_time`'s new outputs follow the
   `HttpResult` convention — a concrete `Ok`/`Err` enum, never
   `std::result::Result`, which type generation cannot emit — and the
   capabilities chapter now recommends it. It is a convention, not a rule the
   compiler enforces.

Naming is settled: the markers live at `crux_core::operation::{Notify, Request,
Stream}` and are used through the module path.

## Next steps

Steps 1 to 3 are done — the traits, the derive, the tightened constructors, the
per-operation rewrites of `crux_kv` and `crux_time`, and kind and handler
emission for all four languages all shipped in the compat release, and two
examples moved across on both sides of the boundary. What remains is the
breaking release:

1. Move the kind to `type Kind` and make the markers blanket impls, so the
   wrong constructor is an ordinary `cargo check` error rather than an E0080 on
   build.
2. Tighten the `Command` and `CommandContext` bounds to the markers, with
   `#[diagnostic::on_unimplemented]` messages.
3. Remove the deprecated enum APIs and the legacy `None` handling in the bridge
   and in type generation.
4. Migrate the remaining four examples and their shells, including the
   FFI-subset-enum question in `counter-routing`.
