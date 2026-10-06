# Type generation

## Why type generation?

Declaring every type across an FFI boundary is painful. Complex types
like nested enums, generics, and rich view models are awkward to expose
directly through general-purpose FFI binding tools. And even when you
_can_ declare them, maintaining the
declarations by hand as your app evolves is tedious and error-prone.

Crux sidesteps this problem by keeping the FFI surface as small as
possible. The entire core-shell interface is just three methods —
`update`, `resolve`, and `view` — and all data crosses the boundary as
serialized byte arrays (using [`bincode`](https://docs.rs/bincode)). The
shell doesn't need to know the Rust types at the FFI level at all.

BoltFFI gives Crux the bindings for that byte-oriented API, but it
doesn't remove the need for generated shell types. Two constraints
matter here:

- Shell types should be immutable value types. Rust-backed FFI objects
  can make ownership and mutation part of the UI boundary; immutability
  is still being worked through in
  [boltffi#292](https://github.com/boltffi/boltffi/issues/292).
- Shells need to connect view models to UI-native state mechanisms:
  Swift `@Observable`, Kotlin `StateFlow`, TypeScript framework state
  such as React `useState`, and C#
  `INotifyPropertyChanged`/`ObservableObject`. Those APIs expect native
  values or native observable wrappers, not Rust-backed objects.

Crux is still exploring where those responsibilities should sit, and
whether [`difficient`](https://github.com/redbadger/difficient/tree/main)
can reduce the payload over the wire by sending changes instead of
whole values. For now, type generation is the stable layer that gives
shells native value types while the FFI stays small.

That generated layer has a concrete job: the shell must serialize
events and deserialize effects and view models on its side of the
boundary. To do that, it needs equivalent type definitions in Swift,
Kotlin, TypeScript, or C#, along with the matching serialization code.
Type generation inspects your Rust types and generates those foreign
types and their `bincode` serialization implementations automatically.

## How it works

Type generation uses the [Facet](https://docs.rs/facet) crate for
zero-cost reflection. Types that derive the `Facet` trait can be
introspected at build time to discover their shape — fields, variants,
generic parameters. The
[facet-generate](https://github.com/redbadger/facet-generate) crate
uses that reflection data to generate equivalent types (and their
serialization code) in Swift, Kotlin, TypeScript, and C#.

The process has three parts:

1. **Annotate your types** — derive `Facet` on types that cross the
   FFI boundary, and use `#[effect(facet_typegen)]` on your `Effect`
   enum.
2. **Add a codegen binary to your shared crate** — a short `main`
   that registers your app and generates the foreign code.
3. **Run it** — typically via a `just typegen` recipe as part of your
   build workflow.

## Annotating your types

### Events, ViewModel, and other data types

Types that the shell needs to know about should derive `Facet` (along
with `Serialize` and `Deserialize` for the FFI serialization). Here's
the counter example:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:10:16}}
```

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:29:32}}
```

Note the `#[repr(C)]` on the enum — this is required by Facet for
enums that cross the FFI boundary.

### The Effect type

The `Effect` enum uses the `#[effect(facet_typegen)]` attribute, which
tells the `#[effect]` macro to generate the type registration code
that the codegen binary needs:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/app.rs:18:22}}
```

The macro discovers the operation types carried by each variant (e.g.
`RenderOperation`) and registers them for type generation
automatically. It also records, per variant, the operation kind the
operation declares and the `Format` of its `Output` — that's the data
behind the [operation kinds and handler API](#operation-kinds-and-the-effect-handler-api)
below.

### Skipping and opaque types

Not all event variants need to cross the FFI boundary. Internal
events (ones the shell never sends) can be excluded from the generated
output with `#[facet(skip)]`:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/app.rs:38:60}}
```

In this example, `Set`, `Update`, and `UpdateBy` are internal events
— the shell never creates them, so they're skipped.

However, `Facet` must still be derivable on the _entire_ type,
including skipped variants. If a skipped variant contains a field
whose type doesn't implement `Facet` (like `crux_http::Result<...>`),
you need to mark that field with `#[facet(opaque)]` so the derive
succeeds. That's why `Set` has both `#[facet(skip)]` on the variant
and `#[facet(opaque)]` on its field.

## The codegen binary

Each shared crate includes a small binary that drives the type
generation. Here's the one from the counter example:

```rust,no_run,noplayground
{{#include ../../../examples/counter/shared/src/bin/codegen.rs}}
```

The key steps are:

1. **`TypeRegistry::new().register_app::<Counter>()?`** — discovers
   all types reachable from your `App` implementation (events, effects,
   view model, and the operation types they reference).
2. **`.build()?`** — produces a `CodeGenerator` with the full type
   graph.
3. **`Config::builder(name, &output_dir)`** — configures the output.
   The `name` parameter is the package/module name (e.g. `"App"` for
   Swift, `"com.crux.examples.counter"` for Kotlin, `"app"` for
   TypeScript, `"CounterApp.Shared"` for C#).
4. **`.swift(&config)?`** / **`.kotlin(&config)?`** /
   **`.typescript(&config)?`** / **`.csharp(&config)?`** — generates
   the code, including the target-language serialization runtime for
   `bincode`.

BoltFFI binding generation is run separately by the shell build recipes with
`boltffi pack ...`. The codegen binary is intentionally focused on Crux app
types; the one thing it can be told about BoltFFI is where its output
lives, with `.boltffi(BoltFfi::new()...)` on the `CodeGenerator`, so that
the generated `Core` can be constructed over it without a hand-written
adapter — see [Bridging to BoltFFI](#bridging-to-boltffi).

### Cargo.toml setup

The codegen binary needs a few additions to your `shared/Cargo.toml`.

Declare the binary, gated on a `codegen` feature:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen_bin}}
```

Enable `facet_typegen` in `crux_core`:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen}}
```

And add `facet` as a dependency — all types that cross the FFI
boundary derive `Facet`:

```toml
{{#include ../../../examples/counter/shared/Cargo.toml:typegen_deps}}
```

## Running type generation

Type generation is typically run via [Just](https://just.systems/)
recipes. Each shell runs the codegen binary and writes the output into
a `generated/` directory inside itself. In the counter example, the
layout looks like this:

```text
examples/counter/
├── shared/            # the Crux core
├── apple/
│   └── generated/     # Swift package "App"
├── Android/
│   └── generated/     # Kotlin package "com.crux.examples.counter"
├── web-react-router/
│   └── generated/
│       └── types/     # TypeScript package "app"
└── ...
```

The package names are set in `codegen.rs` via the `Config::builder`
call — see the codegen binary above.

Each shell's `Justfile` has a `typegen` recipe. For example, the Apple
shell runs:

```sh
RUST_LOG=info cargo run \
    --package shared \
    --bin codegen \
    --features codegen,facet_typegen \
    -- \
        --language swift \
        --output-dir generated
```

The `--output-dir` is relative to the shell directory where the recipe
runs — so the generated code lands right where the shell project can
reference it. The TypeScript shells use `generated/types` to keep the
types separate from the wasm package (which lives in `generated/pkg`).

The `generated/` directories are gitignored and regenerated as part of
the build process. Each shell's `build` recipe depends on `typegen`, and
where the codegen is configured to bridge to BoltFFI, `typegen` in turn
depends on the `boltffi pack` recipe, because the generated package
refers to BoltFFI's.

## What gets generated

For each target language, the codegen produces:

- **Type definitions** — enums, structs, and their serialization code,
  matching the shape of your Rust types. For example, `Event`,
  `Effect`, `ViewModel`, and any operation types.
- **Serialization runtime** — Serde and `bincode` implementations in the
  target language, so the shell can serialize events and deserialize
  effects and view models.
- **Helper extensions** — like `Requests.swift`, which provides
  convenience methods for working with effect requests.
- **An operation-kind accessor, a typed effect handler API and a `Core`
  that drives the loop** — see the next sections.

For Swift, Kotlin, TypeScript, and C#, this typegen output sits beside the
BoltFFI-generated binding package for the byte-oriented core API.

## Operation kinds and the effect handler API

A shell holding a `Request { id, effect }` has to know two things that
are not in the bytes: what type to answer with, and *how many times*.
Both are static properties of the operation each `Effect` variant
carries — an operation declares a
[operation kind](../part-2/capabilities.md#one-output-per-operation), notify,
request or stream, and one `Output` — so type generation emits them.

Next to the generated `Effect`, you get:

- an `OperationKind` type and a per-variant accessor, which is `nil` /
  `null` / `undefined` for an operation that declares no kind;
- an `EffectHandler` protocol or interface with one method per variant:
  a notification's method returns nothing, a request's method returns
  the operation's `Output`, a stream's method takes an
  `EffectSink<Output>`, and a legacy variant's method is handed
  `(operation, requestId, resolve)` exactly as before;
- an `EffectDispatcher(handler, resolve)` that calls the right method
  and resolves the request never, once, or once per sink item,
  serializing each output with the generated bincode serializers.

The `resolve` you hand the dispatcher is your own
`(requestId, bytes) -> ()` callback around the core's `resolve` FFI —
the same one you would have called by hand.

Here is what that looks like for an effect with one variant of each
kind, plus a `Legacy` operation that declares nothing.

**Swift**

```swift
public enum OperationKind: Hashable, Sendable { case notify, request, stream }

extension Effect {
    public var operationKind: OperationKind? { /* generated switch */ }
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

**Kotlin**

```kotlin
enum class OperationKind { NOTIFY, REQUEST, STREAM }

val Effect.operationKind: OperationKind?

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

`dispatch` is `suspend`, because a request's handler method may be. Give
each request its own coroutine if one of them can take a while — a timer,
for instance — so the rest are not held up behind it.

**TypeScript**

```typescript
export type OperationKind = "notify" | "request" | "stream";
export function effectOperationKind(effect: Effect): OperationKind | undefined;

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

The generated union already uses `kind` as its discriminant, so the
accessor is the free function `effectOperationKind(effect)` rather than a
property.

**C#**

```csharp
public enum OperationKind { Notify, Request, Stream }

// emitted inside the generated Effect record, which is not partial
public OperationKind? OperationKind { get; }

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

### The generated Core

With the dispatcher doing the resolving, the loop a shell still has to
write around it is the same in every Crux app: serialize the `Event`,
call the core's `update`, deserialize the `Requests`, re-read the view
when a `Render` arrives, dispatch everything else, and when a request
is resolved call the core's `resolve` and process the requests that
come back. Type generation knows every type in that loop, so it emits
it as a `Core` class, next to the handler API.

`Core` talks to the Rust core through a `CoreBridge` protocol (Swift,
Kotlin, TypeScript) or `ICoreBridge` interface (C#) with three
byte-level methods. If BoltFFI generates your bindings, tell the codegen
where they are and type generation implements the protocol for you (see
[Bridging to BoltFFI](#bridging-to-boltffi) below), so a shell constructs
`Core` from nothing but its `EffectHandler`. Otherwise — another binding
generator, a test double, a preview — you implement it yourself around
whatever produces the bytes, and hand it to `Core` with your
`EffectHandler`:

```swift
public protocol CoreBridge: Sendable {
    func update(_ event: [UInt8]) -> [UInt8]
    func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8]
    func view() -> [UInt8]
}

@available(macOS 14.0, iOS 17.0, tvOS 17.0, watchOS 10.0, *)
@Observable
@MainActor
public final class Core {
    public private(set) var view: ViewModel
    public init(bridge: any CoreBridge, handler: any EffectHandler)
    public convenience init(handler: any EffectHandler)   // with a BoltFFI config
    public func update(_ event: Event)
    public func process(_ requests: [Request])
    public func process(bytes: [UInt8])
}
```

```kotlin
interface CoreBridge {
    fun update(event: ByteArray): ByteArray
    fun resolve(id: UInt, output: ByteArray): ByteArray
    fun view(): ByteArray
}

class Core(bridge: CoreBridge, handler: EffectHandler, scope: CoroutineScope) {
    constructor(handler: EffectHandler, scope: CoroutineScope)   // with a BoltFFI config
    val view: StateFlow<ViewModel>
    fun update(event: Event)
    fun process(requests: List<Request>)
    fun process(bytes: ByteArray)
}
```

```typescript
export interface CoreBridge {
    update(event: Uint8Array): Uint8Array;
    resolve(id: uint32, output: Uint8Array): Uint8Array;
    view(): Uint8Array;
}

export class Core {
    view: ViewModel;
    constructor(bridge: CoreBridge, handler: EffectHandler,
                onView: (view: ViewModel) => void);
    static create(handler: EffectHandler,                 // with a BoltFFI config
                  onView: (view: ViewModel) => void): Promise<Core>;
    update(event: Event): void;
    process(requests: Request[]): void;
    processBytes(bytes: Uint8Array): void;
}
```

C# gets `ICoreBridge` and `sealed class Core(ICoreBridge, IEffectHandler)`,
which implements `INotifyPropertyChanged` and raises `PropertyChanged` for
its `View` property, with `Update(Event)`, `Process(IReadOnlyList<Request>)`
and `Process(byte[])`. With a BoltFFI config it also has a
`Core(IEffectHandler)` constructor.

Things worth knowing:

- **`Core` owns `Render`.** It recognizes the variant carrying
  `crux_core::render::RenderOperation`, re-reads the view from the bridge
  when one arrives, keeps it in `view`, and publishes it the way each
  platform expects: `view` is an `@Observable` property in Swift, a
  `StateFlow` in Kotlin, an `onView` callback in TypeScript, and a
  `PropertyChanged` event in C#. The initial view is read in the
  constructor without a notification. Because `Core` handles it, `EffectHandler.render`
  has a default that does nothing (a protocol extension in Swift, a
  default method in Kotlin and C#, an optional `render?` in TypeScript).
  Implement it only if you drive `EffectDispatcher` without `Core`.
- **The view is held, not just forwarded.** That is deliberate: it is
  where diff-based view updates will be applied when they arrive, without
  changing how you use `Core`.
- **`process(bytes)` is for middleware.** A Rust side that pushes
  effects to the shell asynchronously — the `CruxShell.process_effects`
  callback in the middleware examples — can hand those bytes straight to
  `Core`. It tolerates an empty byte array.
- **Concurrency.** The Swift `Core` is `@MainActor`; the dispatcher's
  resolve hops back to the main actor before touching the bridge, as the
  hand-written shells did. The Kotlin `Core` dispatches each request, and
  processes each resolution, in its own coroutine on the scope you pass.
  In C#, `PropertyChanged` may be raised on a thread-pool thread after
  an asynchronous request completes, so marshal to your UI thread in the
  handler.
- **No `Render`, no `Core`.** An effect enum without a `RenderOperation`
  variant has no view loop to own, so only the handler API is emitted
  for it.
- **Turning it off.** `CodeGenerator::without_core()` leaves the handler
  API in place; `without_effect_handlers()` turns off both, because
  `Core` depends on the dispatcher.

### Bridging to BoltFFI

Type generation does not read BoltFFI's output — the two generators are
independent, and the package, module and class names BoltFFI uses are
decisions you made in `boltffi.toml` and `ffi.rs`. Repeat them in the
codegen and type generation emits the bridge for you:

```rust,ignore
let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .build()?
    .boltffi(
        BoltFfi::new()
            .swift("Shared")   // the Swift module; the package is at ../Shared
            .kotlin()          // CoreFfi is in the generated package
            .typescript("shared", PackageLocation::Path("../pkg".into()))
            .csharp(),         // CoreFfi is in the generated namespace
    );
```

Each language is opted in separately; one you do not name gets exactly
the output described above. `BoltFfi::class(..)` renames the exported
class if yours is not `CoreFfi`; `swift_package(..)`, `kotlin_package(..)`,
`typescript_package(..)` and `csharp_namespace(..)` cover bindings that
live somewhere other than the defaults.

For a named language the generated module gains an `FfiBridge` —
`CoreBridge` implemented over `CoreFfi`, bytes in and bytes out, with the
Swift `Data` conversion and the `@unchecked Sendable` declaration where
they belong — and `Core` gains a constructor that takes only the handler:

```swift
let core = Core(handler: WeatherHandler())
```

```kotlin
val core = Core(handler, CoroutineScope(SupervisorJob() + Dispatchers.Main.immediate))
```

```typescript
const core = await Core.create(new WeatherHandler(), setView);
```

```csharp
var core = new Core(new CounterHandler());
```

TypeScript's is an `async` factory because the wasm module loads
asynchronously: `Core.create` awaits the package's `initialized` promise
before touching `CoreFfi`, which is the one thing every hand-written web
shell had to remember. `CoreBridge` and the two-argument constructors are
still emitted, so a preview or a test can hand `Core` a fake, and a shell
whose FFI has a different shape — the middleware examples, whose
`CoreFfi::new` takes a callback — still writes its own adapter.

Two consequences for the build:

- **Swift.** `FfiBridge.swift` imports the BoltFFI module, so the generated
  package now depends on the BoltFFI package: `Package.swift` gains
  `.package(path: "../Shared")` and the target depends on its product. SPM
  requires a dependent package's deployment target to be at least its
  dependency's, and BoltFFI's package declares one, so give the generated
  package a `platforms:` floor to match through the `Config`:

  ```rust,ignore
  Config::builder("App", &out_dir)
      .platform(".iOS(.v16)")
      .platform(".macOS(.v13)")
      .build()
  ```

  Your app target no longer needs to link the BoltFFI package itself; it
  reaches it through the generated one.
- **TypeScript.** The generated `package.json` depends on the BoltFFI
  package (`"shared": "file:../pkg"`), and type generation runs
  `pnpm install` in the generated package, so run `boltffi pack wasm`
  *before* typegen. The Android recipes already pack first; the web
  recipes in the examples were reordered to match.

Kotlin and C# need nothing else when the bindings share the generated
package or namespace, which is how the examples are configured.

Setting `boltffi(..)` when there is no `Core` to bridge — no registered
app, no `Render` variant, or `without_core()` — is reported as an error
rather than silently ignored.

### Resolve errors

The bridge encodes some key properties of the effect into each request
id (which variant of the `Effect` enum the request carries, and whether
the shell resolves it once or many times), so it can catch common
mistakes early. When the shell calls `resolve`, the bridge checks the
id before it deserializes the response, and reports a mismatch as a
`ResolveError` that says what the outstanding request expects and what
the id carries. The bridge's `BridgeError` wraps it, so the message
the shell sees starts with `could not process response:`.

The id is opaque to the shell, so resolve with it exactly as it
arrived. The examples below are for an app whose `Effect` enum is
`Http` followed by `Render`.

#### Resolving a notification

```text
Attempted to resolve a request that is not expected to be resolved.
```

This is `ResolveError::Never`. The request was a notification, such as
`Render`, whose operation kind is `Notify`, and the core does not wait
for a response to a notification. It usually means a hand-written
dispatcher resolves every effect it handles, whatever its kind. Check
the `OperationKind` accessor on the effect, and resolve a `Request`
once and a `Stream` once per item.

#### Resolving with an id for a different effect

```text
Request 1 expects `Http` (variant 0), but response id 0x01000001 carries `Render` (variant 1).
```

This is `ResolveError::WrongEffect`. The core has a request outstanding
under that sequence number, but it was issued for a different variant
of the `Effect` enum from the one the id names. Either the id was
altered on its way back to the core, or it was issued by an earlier
instance of the core and happens to share a sequence number with a
request the current one is waiting on. An effect whose `EffectFFI`
implementation is hand-written may not know its variant names, and
then the message numbers the variants instead of naming them.

#### Resolving with an id of a different kind

```text
Request 1 expects the Request kind, but response id 0x00800001 carries the Stream kind.
```

This is `ResolveError::WrongKind`. The outstanding request expects to
be resolved once and the id says it is a stream, or the other way
round. The effect matches, so the id was most likely altered on its way
back to the core, or it was issued by an earlier instance of the core.

#### Resolving with an effect variant the enum does not have

```text
`shared::Effect` has only 2 variants, but response id 0x09000001 carries variant 9.
```

This is `ResolveError::NoSuchEffect`. The id names a variant beyond the
end of the `Effect` enum, so this core cannot have issued it. It
typically comes from an id altered on its way back to the core, or from
one kept from a build of the core whose `Effect` enum had more
variants.

#### Resolving a request that has finished

```text
Request with id 1 not found.
```

This is `ResolveError::NotFound`, with the id in decimal. Nothing is
outstanding under that id: the request was resolved already (a
`Request` is resolved exactly once), or the id was never issued.
Sequence numbers are not reused when a request completes, so resolving
twice reports this rather than reaching an unrelated later request.

An effect enum can have at most 256 variants, and `#[effect]` rejects a
larger one.

### Notes and escape hatches

- The emission is **additive**. A shell that matches on `Effect` and
  calls `resolve` by hand keeps working unchanged, which is what Crux's
  Rust shells do — the [Leptos shell](../part-2/shell/leptos.md) matches
  the enum directly, because in Rust the match is already as precise as
  a handler interface.
- The Swift protocol and dispatcher carry
  `@available(macOS 10.15, iOS 13.0, tvOS 13.0, watchOS 6.0, *)`,
  because `Task {}` needs those versions and the generated
  `Package.swift` declares no `platforms:`. A package that declares its
  own platforms conforms without repeating the annotation. Note also
  that the generated operation and output types are not `Sendable`, so
  a `@MainActor` type conforming to the `Sendable` `EffectHandler`
  needs a `nonisolated` extension — see the
  [iOS chapter](../part-2/shell/ios.md). The generated `Core` is
  `@Observable`, so it alone carries
  `@available(macOS 14.0, iOS 17.0, tvOS 17.0, watchOS 10.0, *)` and the
  generated file imports `Observation`; a shell with an older deployment
  target keeps the handler API and dispatcher, which stay at the lower
  bar, and drives the loop itself. `CoreBridge` is `Sendable`; an adapter around BoltFFI's
  non-`Sendable` `CoreFfi` class declares itself `@unchecked Sendable`,
  which is sound because the Rust bridge guards its state with mutexes.
  The generated `FfiBridge` carries that declaration; write it yourself
  only on an adapter of your own.
- `OperationKind`, `EffectSink`, `EffectHandler`, `EffectDispatcher`,
  `Core`, `CoreBridge` and `FfiBridge` (and their C# `I`-prefixed forms)
  are reserved names.
  `TypeRegistry::build` fails if one of your shared types or effect
  variants claims one.
- `CodeGenerator::without_core()` turns off `Core` and `CoreBridge`, and
  with them the BoltFFI bridge, which is an error to configure alongside
  it; `CodeGenerator::without_effect_handlers()` turns off those and the
  handler API too, leaving the types you registered and the
  operation-kind accessor, because a shell that dispatches by hand still
  has to know how many times to resolve.
- The generated Kotlin module declares a dependency on
  `kotlinx-coroutines-core` in its `build.gradle.kts`, which `Core`'s
  `StateFlow` and coroutine launches need.
- Operation names collide with standard library types more often than
  you'd expect — `crux_kv`'s `Set` shadows `Set` in Swift, Kotlin and
  TypeScript. Alias it at the import site
  (`import com.example.Set as KeyValueSet`, `import { Set as SetValue }`).
- Facet type generation requires `facet_generate` 0.21 or later.
