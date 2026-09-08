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
automatically. It also records, per variant, the request kind the
operation declares and the `Format` of its `Output` — that's the data
behind the [request kinds and handler API](#request-kinds-and-the-effect-handler-api)
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
types.

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
the build process. Each shell's `build` recipe depends on `typegen`.

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
- **A request-kind accessor and a typed effect handler API** — see the
  next section.

For Swift, Kotlin, TypeScript, and C#, this typegen output sits beside the
BoltFFI-generated binding package for the byte-oriented core API.

## Request kinds and the effect handler API

A shell holding a `Request { id, effect }` has to know two things that
are not in the bytes: what type to answer with, and *how many times*.
Both are static properties of the operation each `Effect` variant
carries — an operation declares a
[request kind](../part-2/capabilities.md#one-output-per-operation), notify,
request or stream, and one `Output` — so type generation emits them.

Next to the generated `Effect`, you get:

- a `RequestKind` type and a per-variant accessor, which is `nil` /
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
public enum RequestKind: Hashable, Sendable { case notify, request, stream }

extension Effect {
    public var requestKind: RequestKind? { /* generated switch */ }
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

`dispatch` is `suspend`, because a request's handler method may be. Give
each request its own coroutine if one of them can take a while — a timer,
for instance — so the rest are not held up behind it.

**TypeScript**

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

The generated union already uses `kind` as its discriminant, so the
accessor is the free function `effectRequestKind(effect)` rather than a
property.

**C#**

```csharp
public enum RequestKind { Notify, Request, Stream }

// emitted inside the generated Effect record, which is not partial
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
  [iOS chapter](../part-2/shell/ios.md).
- `RequestKind`, `EffectSink`, `EffectHandler` and `EffectDispatcher`
  (and their C# `I`-prefixed forms) are reserved names.
  `TypeRegistry::build` fails if one of your shared types or effect
  variants claims one.
- `CodeGenerator::without_effect_handlers()` turns the handler API off
  and keeps the kind accessor.
- Operation names collide with standard library types more often than
  you'd expect — `crux_kv`'s `Set` shadows `Set` in Swift, Kotlin and
  TypeScript. Alias it at the import site
  (`import com.example.Set as KeyValueSet`, `import { Set as SetValue }`).
- Facet type generation requires `facet_generate` 0.21 or later.
