# RFC: Shell handlers shipped with capabilities

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and the
[generated `Core` RFC](./generated-core.md), and is not yet implemented. It is
written against the compat release of per-operation types, but nothing in it
depends on the `Operation` trait, which the breaking release replaces with
`Notify`, `Request` and `Stream`.
```

This RFC proposes that a capability crate can ship the shell side of its
protocol — a Swift, Kotlin, TypeScript and C# implementation of its effect
handlers — as source embedded in the crate, and that type generation emits
that source into the app's generated package and wires it into the generated
`EffectHandler` as the default behaviour for the operations it covers. An app
that is happy with the defaults writes nothing for those operations. An app
that wants something else configures the shipped implementation, replaces one
method, or replaces the whole implementation, each with one ordinary
declaration.

## Summary

After the per-operation types RFC, a shell implements the generated
`EffectHandler`: one method per effect variant, each with a typed operation in
and a typed output out. After the generated `Core` RFC, that is all a shell
writes. But for the operations of a published capability — `crux_http`,
`crux_kv`, `crux_time` — what it writes is the same in every app: turn an
`HttpRequest` into a `URLRequest`, map the response and its errors into an
`HttpResult`; keep a table of timers keyed by id and cancel one on `Clear`.
The capability author is the one who knows the protocol's rules, and today
those rules are re-derived by every shell author from the book and the
examples.

The proposal:

```rust
// crux_http/src/protocol.rs

#[cfg(feature = "facet_typegen")]
pub static HTTP: ShellHandler = ShellHandler {
    name: "Http",
    swift: Some(ShellSource {
        source: include_str!("../shell/swift/Http.swift"),
        default: Some("URLSessionHttpHandler.shared"),
    }),
    kotlin: Some(ShellSource {
        source: include_str!("../shell/kotlin/Http.kt"),
        default: Some("UrlConnectionHttpHandler"),
    }),
    typescript: Some(ShellSource {
        source: include_str!("../shell/typescript/http.ts"),
        default: Some("fetchHttpHandler"),
    }),
    csharp: Some(ShellSource {
        source: include_str!("../shell/csharp/Http.cs"),
        default: Some("HttpClientHttpHandler.Shared"),
    }),
};

#[derive(Operation, Facet, Serialize, Deserialize)]
#[operation(request, output = HttpResult, shell(HTTP, method = "request"))]
pub struct HttpRequest { /* … */ }
```

and, for the weather app's Swift shell, the handler shrinks to the operations
only that app knows how to perform:

```swift
struct WeatherHandler: EffectHandler {
    func getLocation(_ operation: GetLocation) async -> Location { /* … */ }
    func isLocationEnabled(_ operation: IsLocationEnabled) async -> Bool { /* … */ }
    func secretFetch(_ operation: Fetch) async -> SecretFetchResponse { /* … */ }
    // http, kvGet, kvSet, timeNotifyAfter, timeClear: shipped defaults
}
```

## Why?

### What is left in the shell is mostly protocol, not platform

The weather example's Swift shell, after adopting the generated `Core`, has
193 lines of handler code. Of those, 153 implement `crux_http`, `crux_kv` and
`crux_time` — `http.swift` (52), `keyValue.swift` (44) and `time.swift`
(57) — and would be identical in any other app using those capabilities. The
Kotlin shell has the same 175 lines in `HttpHandler.kt`, `KeyValueHandler.kt`
and `TimeHandler.kt`. The notes example's TypeScript shell carries its own
key-value and timer handlers, down to the same doc comment about a cleared
timer. So does every Crux app outside this repository.

Those files are not hard, but they encode decisions a shell author should not
have to make: that a `URLError.timedOut` becomes `HttpError.timeout` and every
other `URLError` becomes `HttpError.io`; that `NotifyAfter` is answered with
the timer's id when it fires, and that if `Clear` arrives first the timer is
cancelled and the late answer is harmless because the core has stopped
listening. That last rule was settled in the per-operation types RFC's
[review](./per-operation-types.md) and is written into a doc comment in the
weather shell's `time.swift`. It should be written once, by the capability
author, and shipped.

### The wiring already exists for one operation

The generated `Core` RFC gave `render` a default implementation in the
generated `EffectHandler`, because `Core` handles that variant itself. That
default is a protocol extension in Swift, a default body in Kotlin and C# and
an optional member in TypeScript. It is exactly the mechanism a shipped
handler needs: a method the app *may* implement, with behaviour when it does
not. This RFC generalises it from one variant with a fixed body to any
variant whose operation ships a body.

### The default has to be easy to leave and easy to return to

A generated default that is hard to override is worse than no default. Real
shells configure their HTTP client (certificate pinning, a shared
`URLSession`, an `OkHttpClient` with interceptors), put their key-value store
somewhere specific, and sometimes replace a whole capability's implementation
with one backed by a platform library. Each of those has to be one
declaration away from the zero-configuration path, and the compiler has to
point at what is missing when configuration is required.

## Goals

- A capability crate can ship shell handlers for Swift, Kotlin, TypeScript
  and C#, versioned with the crate, so that regenerating types also brings the
  matching handler.
- An app using only shipped defaults writes no handler code for those
  operations. Its `EffectHandler` names only the operations it defines itself.
- Configuring a shipped handler, replacing one of its methods, and replacing it
  wholesale are each a single declaration in the app's handler type, and the
  three compose.
- Where a capability has no sensible zero-configuration default, the compiler
  says which property the app must provide.
- Type generation stays independent of package registries and of BoltFFI. The
  shipped source compiles against the app's own generated types, in the same
  module.
- Nothing changes for a shell that keeps implementing the methods itself.

## Non-goals

- Publishing shell packages to SwiftPM, Maven, npm or NuGet. That is a
  different mechanism with a different cost model, discussed under
  Alternatives.
- Shipping handlers for an app's own operations. The mechanism is available to
  any crate that declares operations, but its purpose is published
  capabilities.
- UI or dependency-injection frameworks. A shipped handler is a plain type; how
  an app constructs and provides it is the app's business.
- The Rust side of the FFI, as in the generated `Core` RFC.

## Design

### Declaring a shipped handler

A capability declares one `ShellHandler` per capability, not per operation.
The unit of implementation on the shell side is the capability — an HTTP
client, a key-value store, a timer table — because its operations share
state, and because that is the unit an app would want to configure or replace.

```rust
pub struct ShellHandler {
    /// UpperCamelCase. Names the shipped protocol (`HttpHandler`,
    /// `IHttpHandler` in C#) and the property the generated `EffectHandler`
    /// exposes it through (`httpHandler`).
    pub name: &'static str,
    pub swift: Option<ShellSource>,
    pub kotlin: Option<ShellSource>,
    pub typescript: Option<ShellSource>,
    pub csharp: Option<ShellSource>,
}

pub struct ShellSource {
    /// The source text, emitted verbatim into the generated module.
    pub source: &'static str,
    /// An expression yielding the default instance, or `None` if the
    /// capability has no zero-configuration implementation on this platform.
    pub default: Option<&'static str>,
}
```

Each operation names the handler it belongs to and the method on the shipped
protocol that performs it:

```rust
#[operation(request, output = ValueResult, shell(KEY_VALUE, method = "get"))]
pub struct Get { pub key: String }
```

`method` defaults to the lower-camel-cased operation type name, so `Get` maps
to `get` and `NotifyAfter` to `notifyAfter`; `HttpRequest` spells it out as
`request`. A shipped protocol method has the same shape as the generated
`EffectHandler` method for that variant: a request returns its output, a
stream takes an `EffectSink`, a notification returns nothing. A legacy
operation with no declared kind cannot be bound; the derive rejects
`shell(..)` without one of `notify`, `request` or `stream`.

A language a capability does not ship (`kotlin: None`) simply gets no default
on that platform: the app implements the method as it does today.

### Recording, without touching the trait

The derive does not add anything to the `Operation` trait. It adds one call to
the typegen registration it already generates:

```rust
fn register_types_facet(generator: &mut TypeRegistry) -> Result<_, _> {
    generator
        .register_type::<Self>()?
        .register_type::<Self::Output>()?
        .register_shell_handler::<Self>(&KEY_VALUE, "get")
}
```

`TypeRegistry` keeps the bindings keyed by the operation's `TypeId`.
`EffectMeta::variant::<Op>()` already reads `TypeId::of::<Op>()` to set
`render`; it now records the id, and `TypeRegistry::build` joins each variant
to its binding. This is why the breaking release's removal of `Operation` in
favour of `Notify`, `Request` and `Stream` does not affect the design: the
binding travels with the registration call, which the derive emits whichever
trait it is implementing, and the registry needs nothing from the trait but
the type.

`build` also checks that two bindings naming the same `ShellHandler::name`
point at the same static, and that no name collides with a registered type or
a reserved name.

The static and `register_shell_handler` are gated on `facet_typegen`, as the
registration already is, so the source text is compiled into the typegen
binary and not into the app's core. This relies on the capability crate
having a feature of that name, which is the existing convention for
`register_types_facet`.

### What is emitted

For each shipped handler with source for the target language, the plugin
emits two things.

**The shipped source**, as a companion file in the generated module: Swift
`Sources/App/Http.swift`, Kotlin `com/crux/example/weather/Http.kt`, C#
`Company/Shared/Http.cs`. Type generation prepends the module header the
language needs — the `package` line in Kotlin, the file-scoped `namespace` in
C# — and writes the source otherwise verbatim. The source therefore refers to
`HttpRequest` and `HttpResult` unqualified, as any file in that module would.
TypeScript is one file per module, so there the source is appended to the
module, after the types, the way `Core` is today.

**The wiring**, in the primary effect's module next to `EffectHandler`:

- a property on `EffectHandler` exposing the handler, named
  `<name>Handler`, typed as the shipped protocol, defaulted to the declared
  `default` expression when there is one;
- a default body for every bound method, delegating to that property.

In Swift:

```swift
public protocol EffectHandler: Sendable {
    var httpHandler: any HttpHandler { get }
    var timeHandler: any TimeHandler { get }
    func http(_ operation: HttpRequest) async -> HttpResult
    func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId
    func timeClear(_ operation: Clear) async -> TimerId
    func getLocation(_ operation: GetLocation) async -> Location
    // …
}

extension EffectHandler {
    /// The shipped `crux_http` implementation. Override to configure it, or
    /// implement `http(_:)` to replace it for this operation.
    public var httpHandler: any HttpHandler { URLSessionHttpHandler.shared }
    public func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }
    public var timeHandler: any TimeHandler { TaskTimeHandler.shared }
    public func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId {
        await timeHandler.notifyAfter(operation)
    }
    public func timeClear(_ operation: Clear) async -> TimerId {
        await timeHandler.clear(operation)
    }
}
```

In Kotlin, the same as default members of the interface:

```kotlin
interface EffectHandler {
    val httpHandler: HttpHandler get() = UrlConnectionHttpHandler
    suspend fun http(operation: HttpRequest): HttpResult = httpHandler.request(operation)
    // …
}
```

In C#, as default interface members on `IEffectHandler`, with the property
named `HttpHandler` and the protocol `IHttpHandler`.

In TypeScript the interface cannot carry defaults, so both members are
optional and the generated `EffectDispatcher` falls back:

```ts
export interface EffectHandler {
  httpHandler?: HttpHandler;
  http?(operation: HttpRequest): Promise<HttpResult>;
  // …
}

// in EffectDispatcher
const output = this.handler.http
  ? await this.handler.http(operation)
  : await (this.handler.httpHandler ?? fetchHttpHandler()).request(operation);
```

A generated doc comment on every defaulted member says which capability it
comes from and names the two ways to override it, so the answer to "where does
this behaviour come from" is on hover.

### The developer experience, in tiers

Every tier is a declaration in the app's handler type, and they compose per
capability.

**Nothing.** The app's handler implements only the operations it defines.
Shipped handlers with defaults do their work.

```kotlin
class WeatherHandler(private val location: LocationClient) : EffectHandler {
    override suspend fun getLocation(operation: GetLocation) = location.current()
    override suspend fun isLocationEnabled(operation: IsLocationEnabled) = location.enabled()
}
```

**Configure.** Override the property with a configured instance of the shipped
implementation, or any other type conforming to the shipped protocol.

```kotlin
override val httpHandler = OkHttpHttpHandler(client)          // app's own conformer
override val keyValueHandler = FileKeyValueHandler(context.filesDir)
```

```swift
let httpHandler: any HttpHandler = URLSessionHttpHandler(session: pinnedSession)
```

**Replace one method.** Implement the method. The shipped implementation still
serves the capability's other operations.

```swift
func kvGet(_ operation: Get) async -> ValueResult {
    if operation.key == "session" { return await keychainGet() }
    return await keyValueHandler.get(operation)
}
```

**Replace the capability.** Provide a conformer of the shipped protocol, as
under Configure. The protocol is also the natural seam for a fake in tests.

**Required configuration.** When a capability declares no default for a
platform, the property has no default and the compiler reports the
non-conformance with the property's name. `crux_kv` on the JVM is the
expected case: the generated Kotlin module is a plain JVM library with no
Android APIs, so a store needs a directory it cannot guess.

```text
Class 'WeatherHandler' is not abstract and does not implement abstract member
'keyValueHandler'
```

### State, defaults and concurrency

The `default` expression is evaluated on every access to the property, so it
has to denote a stable instance: a `static let shared` in Swift, an `object`
in Kotlin, a static in C#, a module-level lazily-initialised value in
TypeScript. The lazy form matters in TypeScript because the source is
appended to a module whose class declarations are not hoisted; a top-level
`new` before the class is declared would fail at load.

Shipped Swift protocols are `Sendable` and their methods `nonisolated`, matching
`EffectHandler`; a stateful implementation guards its state with an actor or
a lock, as the weather shell's timer table does today with `@MainActor`.
Kotlin implementations are `suspend` and choose their own dispatcher, as the
weather shell's `withContext(Dispatchers.IO)` does. Nothing about the generated
`Core` or `EffectDispatcher` changes.

### Rules for shipped source

These are constraints on capability authors, checked by review and by the
examples' CI rather than by type generation:

- Depends only on the platform standard library and what the generated
  module already requires: Foundation, the JDK and `kotlinx-coroutines-core`,
  the browser or Node globals, the BCL. A capability that genuinely needs a
  library can add it through facet-generate's existing
  `manifest_dependencies`, but every app then pays for it, so the bar is high.
- The Kotlin source is JVM, not Android. No `android.*`.
- Declares a protocol named `<Name>Handler` (`I<Name>Handler` in C#) whose
  methods match the bound operations' shapes, and the types the `default`
  expressions name. Type generation does not parse the source; the shell's
  compiler is the check, and the example shells are where it runs on every
  change.
- Declares nothing else at module scope under a name a generated type could
  take. `<Name>Handler` joins the reserved names.
- Does not log through an app-specific logger. A shipped handler either stays
  quiet or exposes a hook on its protocol.

The source lives under `shell/<language>/` in the capability crate, beside the
Rust that defines the protocol it implements, and changes to the two are
reviewed together.

### Changes to facet-generate

One addition. The `EmitterPlugin` trait gains a hook for **module companion
files**: files a plugin wants written into the module's own directory —
`Sources/<Module>/`, the Kotlin package directory, the C# namespace
directory — with the module's header prepended by the installer. Unlike
`runtime_files`, which the installers skip when the serde runtime comes from
an external package, companion files are written whenever the module itself
is. They are the right home for shipped sources in Swift, Kotlin and C#, where
one file per capability is what a reader expects to find. TypeScript's
single-file modules do not need it.

Everything else uses hooks that exist: `after_type` for the wiring, as the
handler and `Core` plugins do, and `manifest_dependencies` for the rare
library.

### Names and escape hatches

`CodeGenerator::without_shell_handlers()` emits the handler API without the
shipped sources or the wiring, for a shell that wants the pre-RFC shape.
`without_effect_handlers()` implies it.

## Drawbacks

**Foreign-language source inside Rust crates.** Four files per capability that
`cargo test` cannot compile. Their tests are the example shells, which means a
change to a shipped file is verified only where an example uses that
capability on that platform. The examples are already the shells' CI, and the
weather example covers all three published capabilities in Swift, Kotlin and
TypeScript, but the coverage is indirect and today it has a hole: the only C#
shell in the repository is the counter example's, which uses none of them. A
C# shell for counter-http, or a typegen fixture that compiles the generated
package, is part of the implementation.

**Four implementations to keep in step.** A protocol change now touches Rust,
Swift, Kotlin, TypeScript and C# in one pull request. That is the point — the
author who changes the protocol is the one who knows what the shells must do —
but it raises the cost of contributing a capability.

**A generated default does real work.** The generated `Core` RFC noted that a
defaulted `render` silently does nothing. A defaulted `http` silently does
networking. That is what the app asked for by using `crux_http`, and the doc
comment says where it comes from, but an author reading their own handler
type no longer sees every effect it produces.

**Required properties are all-or-nothing.** An app that implements every
method of a capability itself, on a platform where that capability has no
default, still has to satisfy the property. It can do so with a throwing
getter, which is ugly. See open question 1.

**Swift needs the property's type spelled out.** A stored property witnessing
`var httpHandler: any HttpHandler` must be declared with that type; `let
httpHandler = URLSessionHttpHandler(session:)` does not conform. One
annotation, but a surprising one.

**Stdlib-only defaults are sometimes the second-best implementation.** The
weather Android shell uses OkHttp today and would keep using it, through the
Configure tier, rather than the shipped `HttpURLConnection` default. The
default exists to make the first build work, not to be the last word.

## Migration

Nothing is required. A shell that implements `http`, `kvGet` and the rest keeps
compiling and keeps its behaviour, because an implemented method wins over the
default.

To adopt, delete the method and the file behind it, and either accept the
default or override the property with a configured instance. The notes and
weather examples do this in the implementing pull request, so the book's
Part II shows handlers that mention only app-defined operations.

The [migration guide](../guide/migrate-per-operation-types.md) gains a section
per language.

## Alternatives considered

**Publish shell packages.** Give the capability types a facet namespace, use
facet-generate's `ExternalPackage` to import them from a published SwiftPM,
Maven, npm and NuGet package, and ship the handler in that package with native
tests. Real libraries with their own release cadence, but four registries to
publish to in lockstep with the crate, a breaking move of every capability
type out of the app's namespace, and the bincode runtime shared across
packages. Worth revisiting when shipped handlers are large enough to need
native unit tests; this RFC's protocol-plus-property shape does not preclude
it, since a published package could provide the conformer the property takes.

**A byte-level seam.** Publish a package that bundles its own copy of the
types and exposes `handle(bytes) -> bytes`; type generation emits a default
that serialises across the seam. Avoids the namespace move but keeps the four
registries, and adds a protocol-version check the app can get wrong.

**A shared static instead of a property.** Emit the default method as
`URLSessionHttpHandler.shared.request(op)` with no property on
`EffectHandler`. Simpler to generate and to read, but configuration means
either mutating a global or re-implementing the delegation, and there is no
compile-time signal for a capability that needs configuring. Rejected because
the property is what makes the Configure tier a single declaration.

**One handler per operation.** Bind each operation to its own shipped type
rather than grouping by capability. Loses the shared state a timer table or a
store needs, and multiplies the properties an app would configure.

**Runtime registration.** A `ShellHandlers.register(HttpHandler.self, …)` the
app calls at startup, consulted by the default methods. Discoverable only by
reading generated code, fails at runtime when missing, and invisible to the
compiler. Rejected.

**Documentation.** Keep the handler files in the book and the examples and
tell people to copy them. This is the status quo, and its failure mode is the
one described under Why: rules re-derived per app, drifting as the protocol
evolves.

## Open questions

1. **Softening required properties.** For a capability with no default, should
   the property instead be optional with a trapping fallback — a runtime error
   at first use naming the property and the methods — so that an app which
   implements every method itself need not declare a dead property? Compile
   time is the better default; the question is whether the corner case is
   common enough to pay for it.
2. **Logging.** Shells want to see HTTP traffic and timer activity in their own
   logs. A shipped protocol could carry an optional observer, or the shipped
   implementations could take a logger in their configuration. Left to the
   first implementation.
3. **Which defaults.** Tentatively: `crux_http` defaults everywhere
   (`URLSession`, `HttpURLConnection`, `fetch`, `HttpClient`); `crux_time`
   defaults everywhere; `crux_kv` defaults to `UserDefaults` in Swift,
   `localStorage` where present in TypeScript, and requires configuration on
   the JVM and in C#. To be settled per capability in the implementing pull
   requests.
4. **Renamed variants.** The wiring uses the variant's emitted name, so a
   renamed variant still binds. If two variants of one effect bind the same
   shipped method — two `HttpRequest` variants — both delegate to it, which is
   correct but worth a note in the book.

## Next steps

1. Add the companion-file hook to facet-generate and release it.
2. Add `ShellHandler`, `register_shell_handler`, the `shell(..)` derive
   attribute and the plugin to `crux_core`, with `without_shell_handlers()`.
3. Ship handlers for `crux_http`, `crux_kv` and `crux_time` in all four
   languages, and migrate the notes and weather shells to them, so CI compiles
   every shipped Swift, Kotlin and TypeScript file. Give counter-http a C#
   shell, or add a C# typegen fixture, so the C# files are compiled too.
4. Write the capability-author chapter: how to declare a shipped handler, the
   rules above, and how the examples test it.
