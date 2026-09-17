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
handlers — as source embedded in the crate, and that an app can ask type
generation to emit that source into its generated package, together with a
small generated adapter that connects it to the generated `EffectHandler`.
The app chooses, in two places, whether to use it: once in its type generation
configuration, which decides what is emitted, and once in its handler type,
which decides what is wired. An app that makes neither choice gets exactly
what it gets today. An app that makes both writes nothing else for the
operations the shipped handler covers, and can configure the shipped
implementation, replace one of its methods, or replace it wholesale, each with
one ordinary declaration.

## Summary

After the per-operation types RFC, a shell implements the generated
`EffectHandler`: one method per effect variant, each with a typed operation in
and a typed output out. After the generated `Core` RFC, that is all a shell
writes. For the operations of a published capability — `crux_http`,
`crux_kv`, `crux_time` — most of what it writes encodes rules that belong to
the protocol, not to the app: that a `URLError.timedOut` becomes
`HttpError.timeout` and every other `URLError` becomes `HttpError.io`; that
`NotifyAfter` is answered with the timer's id when it fires, and that if
`Clear` arrives first the timer is cancelled and the late answer is harmless.
The capability author is the one who knows those rules, and today every shell
author re-derives them from the book and the examples.

Which HTTP client performs the request, on the other hand, is the shell's
decision, and some shells have strong reasons to make it themselves. So the
proposal is a reference implementation the shell can adopt, not a default it
inherits.

The capability declares what it ships:

```rust
// crux_http/src/lib.rs

#[cfg(feature = "facet_typegen")]
pub static HTTP: ShellHandler = ShellHandler {
    name: "Http",
    methods: &[ShellMethod::new::<HttpRequest>("request")],
    swift: Some(ShellSource::stdlib(include_str!("../shell/swift/Http.swift"))),
    kotlin: Some(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt"))),
    typescript: Some(ShellSource::stdlib(include_str!("../shell/typescript/http.ts"))),
    csharp: Some(ShellSource::stdlib(include_str!("../shell/csharp/Http.cs"))),
};
```

The app asks for it where it configures type generation:

```rust
// shared_types/build.rs

let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .build()?
    .shell_handler(&crux_http::HTTP)
    .shell_handler(&crux_time::TIME);
```

and its handler adopts it by conforming:

```swift
struct WeatherHandler: EffectHandler, HttpHandling, TimeHandling {
    let httpHandler: any HttpHandler = URLSessionHttpHandler.shared
    let timeHandler: any TimeHandler = TaskTimeHandler.shared

    func getLocation(_ operation: GetLocation) async -> Location { /* … */ }
    func isLocationEnabled(_ operation: IsLocationEnabled) async -> Bool { /* … */ }
    func secretFetch(_ operation: Fetch) async -> SecretFetchResponse { /* … */ }
}
```

`http`, `timeNotifyAfter` and `timeClear` are satisfied by `HttpHandling` and
`TimeHandling`. Drop a conformance and the compiler asks for the methods
again.

## Why?

### The rules are protocol; the client is platform

The weather example's Swift shell, after adopting the generated `Core`, has
193 lines of handler code. Of those, 153 implement `crux_http`, `crux_kv` and
`crux_time` — `http.swift` (52), `keyValue.swift` (44) and `time.swift`
(57). The Kotlin shell has the same 175 lines in `HttpHandler.kt`,
`KeyValueHandler.kt` and `TimeHandler.kt`. The notes example's TypeScript
shell carries its own key-value and timer handlers, down to the same doc
comment about a cleared timer.

Two different things are tangled in those files. One is the platform call:
`URLSession.data(for:)`, `HttpURLConnection`, `fetch`. The other is the
protocol: how a response and its errors map onto `HttpResult`, what a timer
answers with and when, what a cleared timer does. The platform call is the
shell's business, and a shell with a hardened proxy, a pinned session or a
corporate HTTP stack should be able to keep it. The protocol rules are the
capability author's business, and they should be written once, next to the
Rust that defines them, and shipped. The rule about the late timer answer was
settled in the per-operation types RFC's [review](./per-operation-types.md)
and today lives in a doc comment in the weather shell's `time.swift`, which
is the wrong place for it.

A shipped implementation carries the rules. Whether to use it, and which
client to use it with, stays with the shell.

### The choice belongs to the shell

Nothing about this should be decided in the core. A capability crate knows
how its protocol works and can offer an implementation; it cannot know
whether a given app wants one, and it must not put behaviour into a shell the
shell did not ask for. So the design has two explicit opt-ins and no implicit
ones:

- **Emission** is opt-in per capability, in the app's type generation
  configuration. An app that does not register a shipped handler gets no
  source, no adapter and no manifest dependency for it. An app that drives
  `EffectDispatcher` itself, or matches on `Effect` by hand, registers
  nothing and sees nothing.
- **Wiring** is opt-in per capability, in the app's handler type. The
  generated `EffectHandler` is unchanged: every method is still abstract.
  What is generated alongside it is an adapter protocol the app may conform
  to. Conforming supplies the methods; not conforming leaves them for the
  app to implement.

An app reading its own handler type sees every effect it produces, either as
a method it wrote or as a conformance it declared.

### The default has to be easy to leave and easy to return to

A shipped implementation that is hard to configure is worse than none. Real
shells configure their HTTP client (certificate pinning, a shared
`URLSession`, an `OkHttpClient` with interceptors), put their key-value store
somewhere specific, and sometimes replace a whole capability's implementation
with one backed by a platform library. Each of those has to be one
declaration away from the plain path, and the compiler has to point at what
is missing.

## Goals

- A capability crate can ship shell handlers for Swift, Kotlin, TypeScript
  and C#, versioned with the crate, so that regenerating types also brings the
  matching handler.
- The core makes no decision about the shell. What is emitted, and what is
  wired, are both chosen by the app, and nothing is emitted for a capability
  the app did not ask for.
- An app that adopts a shipped handler writes, per capability, one
  registration in type generation and one conformance with one property in
  its handler type. Its `EffectHandler` methods for that capability are
  supplied.
- Configuring a shipped handler, replacing one of its methods, and replacing it
  wholesale are each a single declaration in the app's handler type, and the
  three compose.
- Type generation stays independent of package registries and of BoltFFI. The
  shipped source compiles against the app's own generated types, in the same
  module.
- Nothing changes for a shell that keeps implementing the methods itself, and
  a shell can stop using a shipped handler by deleting a conformance.

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
    /// `IHttpHandler` in C#), the generated adapter (`HttpHandling`,
    /// `IHttpHandling`) and the property the adapter requires (`httpHandler`).
    pub name: &'static str,
    /// Which operations the shipped protocol performs, and through which
    /// method.
    pub methods: &'static [ShellMethod],
    pub swift: Option<ShellSource>,
    pub kotlin: Option<ShellSource>,
    pub typescript: Option<ShellSource>,
    pub csharp: Option<ShellSource>,
}

pub struct ShellMethod {
    /// The operation type, recorded as its `TypeId`.
    operation: fn() -> TypeId,
    /// The method on the shipped protocol that performs it.
    method: &'static str,
}

impl ShellMethod {
    pub const fn new<Op: 'static>(method: &'static str) -> Self { /* … */ }
}

pub struct ShellSource {
    /// The source text, emitted verbatim into the generated module.
    pub source: &'static str,
    /// Manifest entries the source needs beyond the platform standard
    /// library, in the form facet-generate's `manifest_dependencies` takes.
    /// Empty for a handler that needs nothing.
    pub dependencies: &'static [&'static str],
}

impl ShellSource {
    pub const fn stdlib(source: &'static str) -> Self { /* dependencies: &[] */ }
}
```

The binding from operation to method lives in the static, with everything
else the capability says about its shell side, and the operation declaration
does not mention it. Nothing is added to the `Operation` derive or to the
trait, which is why the breaking release's move to `Notify`, `Request` and
`Stream` does not touch this design.

A shipped protocol method has the same shape as the generated `EffectHandler`
method for the variant that carries its operation: a request returns its
output, a stream takes an `EffectSink`, a notification returns nothing. A
legacy operation with no declared kind cannot be bound; type generation
rejects the registration and says which operation.

A language a capability does not ship (`kotlin: None`) simply has nothing to
emit on that platform: the app implements the methods as it does today, and
type generation says so when the handler is registered for that language.

The static is gated on `facet_typegen`, as `register_types_facet` already is,
so the source text is compiled into the typegen binary and not into the app's
core. This relies on the capability crate having a feature of that name,
which is the existing convention.

### Registering it

The app names the shipped handlers it wants on the built `CodeGenerator`,
next to `boltffi`:

```rust
let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .build()?
    .boltffi(BoltFfi::new().swift("Shared").kotlin())
    .shell_handler(&crux_http::HTTP)
    .shell_handler(&crux_time::TIME);
```

`EffectVariantMeta` gains the operation's `TypeId`, which
`EffectMeta::variant::<Op>()` already computes to set `render`. When a
language is generated, the plugin joins each registered handler's `methods`
to the variants that carry those operations, and emits for the matches.

Registration is checked, and the checks say what to do:

- A registered handler none of whose operations appears in any registered
  effect is an error — the app asked for `crux_http::HTTP` but no effect
  variant carries `HttpRequest` — because silently emitting nothing would
  leave the shell hunting for a protocol that is not there. This mirrors
  `check_boltffi`.
- A registered handler together with `without_effect_handlers()` is an error,
  for the same reason `boltffi` is.
- Two registered handlers with the same `name`, or a `name` that collides
  with a registered type or a reserved name, is an error.
- A handler whose source names a dependency reports it through
  `manifest_dependencies`, so the manifest gains it only when the handler is
  registered.

### What is emitted

For each registered handler with source for the target language, the plugin
emits one companion file in the generated module — Swift
`Sources/App/Http.swift`, Kotlin `com/crux/example/weather/Http.kt`, C#
`Company/Shared/Http.cs` — containing two things. In TypeScript, whose
modules are one file, both are appended to the module after the types, the
way `Core` is today.

**The shipped source**, verbatim, after the module header the language needs
(the `package` line in Kotlin, the file-scoped `namespace` in C#). It
declares the protocol `HttpHandler` and one or more implementations of it,
and refers to `HttpRequest` and `HttpResult` unqualified, as any file in
that module would.

**The adapter**, generated, which is the only thing that knows the app's
variant names:

```swift
/// `crux_http`'s shipped handler. Conform to this and supply `httpHandler`
/// to have `http(_:)` performed by it; implement `http(_:)` yourself to
/// take that operation back.
public protocol HttpHandling: EffectHandler {
    var httpHandler: any HttpHandler { get }
}

extension HttpHandling {
    public func http(_ operation: HttpRequest) async -> HttpResult {
        await httpHandler.request(operation)
    }
}
```

In Kotlin, an interface with a default member:

```kotlin
interface HttpHandling : EffectHandler {
    val httpHandler: HttpHandler
    override suspend fun http(operation: HttpRequest): HttpResult =
        httpHandler.request(operation)
}
```

In C#, a derived interface providing the base member's default:

```csharp
public interface IHttpHandling : IEffectHandler
{
    IHttpHandler HttpHandler { get; }
    Task<HttpResult> IEffectHandler.Http(HttpRequest operation) =>
        HttpHandler.Request(operation);
}
```

In TypeScript, where interfaces carry no behaviour, a function returning the
members to spread into the handler object:

```ts
export function httpHandling(handler: HttpHandler): Pick<EffectHandler, "http"> {
  return { http: (operation) => handler.request(operation) };
}
```

The generated `EffectHandler` itself does not change. Every method is still a
requirement; the adapter is one way of meeting it. In all four languages the
app's own implementation of a method takes precedence over the adapter's, so
conforming and implementing compose per method.

### The developer experience, in tiers

Every tier is a declaration in the app's handler type, and they compose per
capability.

**Use.** Conform to the adapter and supply the shipped implementation.

```kotlin
class WeatherHandler(private val location: LocationClient) :
    EffectHandler, HttpHandling, TimeHandling {
    override val httpHandler = UrlConnectionHttpHandler
    override val timeHandler = TaskTimeHandler()
    override suspend fun getLocation(operation: GetLocation) = location.current()
    override suspend fun isLocationEnabled(operation: IsLocationEnabled) = location.enabled()
}
```

```ts
const handler: EffectHandler = {
  ...httpHandling(fetchHttpHandler),
  ...timeHandling(new TaskTimeHandler()),
  getLocation: async (operation) => location.current(),
  isLocationEnabled: async (operation) => location.enabled(),
};
```

**Configure.** Supply a configured instance of the shipped implementation, or
any other type conforming to the shipped protocol.

```kotlin
override val httpHandler = OkHttpHttpHandler(client)            // app's own conformer
override val keyValueHandler = FileKeyValueHandler(context.filesDir)
```

```swift
let httpHandler: any HttpHandler = URLSessionHttpHandler(session: pinnedSession)
```

**Replace one method.** Implement the method. The adapter still serves the
capability's other operations.

```swift
func kvGet(_ operation: Get) async -> ValueResult {
    if operation.key == "session" { return await keychainGet() }
    return await keyValueHandler.get(operation)
}
```

**Replace the capability.** Supply a conformer of the shipped protocol, as
under Configure, or drop the conformance and implement the methods. The
protocol is also the natural seam for a fake in tests.

**Do not use it.** Do not register it. Nothing is emitted, and the handler
type is exactly what the per-operation types RFC generates.

### State and concurrency

The app owns the instance it supplies, so lifetime is the app's. Shipped
implementations that are safe to share offer a shared instance
(`URLSessionHttpHandler.shared`, a Kotlin `object`, a C# static) for the
plain path, and an initialiser for the configured one. Shipped
implementations that hold state, like a timer table, are constructed by the
app, once, where it constructs its handler.

Shipped Swift protocols are `Sendable` and their methods `nonisolated`, matching
`EffectHandler`; a stateful implementation guards its state with an actor or
a lock, as the weather shell's timer table does today with `@MainActor`.
Kotlin implementations are `suspend` and choose their own dispatcher, as the
weather shell's `withContext(Dispatchers.IO)` does. Nothing about the generated
`Core` or `EffectDispatcher` changes.

### Rules for shipped source

These are constraints on capability authors. Type generation does not parse
the source, so they are checked by review and by the example shells' CI, and
they need to be held to firmly, because a shipped file that breaks breaks in
every app that registers it.

- Depends only on the platform standard library and what the generated
  module already requires: Foundation, the JDK and `kotlinx-coroutines-core`,
  the browser or Node globals, the BCL. A capability that genuinely needs a
  library declares it in `ShellSource::dependencies`, so that it lands in the
  manifest only for apps that register the handler, but every one of those
  apps then pays for it, so the bar is high.
- The Kotlin source is JVM, not Android. No `android.*`.
- Declares a protocol named `<Name>Handler` (`I<Name>Handler` in C#) whose
  methods match the bound operations' shapes, and at least one implementation
  of it. Exposes configuration through the implementation's initialiser, not
  through globals.
- Declares nothing else at module scope under a name a generated type could
  take. `<Name>Handler` and `<Name>Handling` join the reserved names.
- Does not log through an app-specific logger. A shipped handler either stays
  quiet or exposes a hook on its protocol.
- Is compiled by an example shell on every change. The weather example covers
  `crux_http`, `crux_kv` and `crux_time` in Swift, Kotlin and TypeScript; a
  C# shell that uses them, or a typegen fixture that compiles the generated
  package, is part of the implementation.

The source lives under `shell/<language>/` in the capability crate, beside the
Rust that defines the protocol it implements, and changes to the two are
reviewed together.

### Changes to facet-generate

None. The hook this design leans on already exists: `EmitterPlugin` has
**module companion files**, files a plugin wants written into the module's
own directory — `Sources/<Module>/`, the Kotlin package directory, the C#
namespace directory — with the module's header prepended. Unlike
`runtime_files`, which the installers skip when the serde runtime comes from
an external package, companion files are written whenever the module itself
is. It was added for the generated `Core`'s BoltFFI bridge, whose `FfiBridge`
is emitted through it, and it is the right home for shipped sources in Swift,
Kotlin and C# too, where one file per capability is what a reader expects to
find. TypeScript's single-file modules do not need it.

Everything else uses hooks that exist: `after_type` for the TypeScript
emission, as the handler and `Core` plugins do, and `manifest_dependencies`
for the rare library.

## Drawbacks

**Foreign-language source inside Rust crates.** Four files per capability that
`cargo test` cannot compile. Their tests are the example shells, which means a
change to a shipped file is verified only where an example registers that
capability on that platform. The examples are already the shells' CI, but the
coverage is indirect and today it has a hole: the only C# shell in the
repository is the counter example's, which uses none of the published
capabilities. A C# shell for counter-http, or a typegen fixture that compiles
the generated package, is part of the implementation.

**Four implementations to keep in step.** A protocol change now touches Rust,
Swift, Kotlin, TypeScript and C# in one pull request. That is the point — the
author who changes the protocol is the one who knows what the shells must do —
but it raises the cost of contributing a capability.

**Three declarations per capability, in two places.** The plain path is a
registration in `build.rs`, a conformance and a property. Each is there
because it is a decision the shell should make visibly, but it is more than
the zero declarations a core-side default would have cost, and the
registration in particular is one the app has to know to make. The
capability's documentation and the book's typegen chapter are where it learns
that; type generation cannot suggest it, because it does not know which
crates ship handlers until they are registered.

**Swift needs the property's type spelled out.** A stored property witnessing
`var httpHandler: any HttpHandler { get }` must be declared with that type;
`let httpHandler = URLSessionHttpHandler.shared` does not conform. One
annotation, but a surprising one.

**Stdlib-only implementations are sometimes the second-best implementation.**
The weather Android shell uses OkHttp today and would keep using it, through
the Configure tier, rather than the shipped `HttpURLConnection` handler. The
shipped implementation exists to make the first build work and to carry the
protocol rules, not to be the last word on the client.

## Migration

Nothing is required. A shell that implements `http`, `kvGet` and the rest keeps
compiling and keeps its behaviour. A shell that registers nothing generates
what it generates today.

To adopt, register the handler in type generation, regenerate, add the
conformance and the property to the handler type, and delete the method and
the file behind it. The notes and weather examples do this in the
implementing pull request, so the book's Part II shows handlers that
implement only app-defined operations.

The [migration guide](../guide/migrate-per-operation-types.md) gains a section
per language.

## Alternatives considered

**A default wired by the core.** The first draft of this RFC had each
operation name its shipped method in the `#[operation]` attribute, and gave
`ShellSource` a `default` expression, so that the generated `EffectHandler`
carried a defaulted property and defaulted methods, and an app that wrote
nothing got the shipped implementation. Review made the case against it: the
core was deciding what the shell does; an app that never wanted the shipped
handler got it emitted anyway, including into shells that drive
`EffectDispatcher` themselves; and a capability with no sensible default on
one platform made the property required there and optional elsewhere. The
adapter shape keeps the one-declaration configuration the default was there
to provide, and moves both decisions to the app.

**Binding on the operation declaration.** Keep `shell(HTTP, method = "request")`
on the operation, as protocol metadata. It is metadata rather than a shell
decision, but it puts a shell-side concern on every operation of the
capability, and it means the derive has to grow for a feature the trait does
not know about. The static already exists to hold everything the capability
says about its shell side; the binding belongs there.

**Generalising the `render` default.** The generated `Core` RFC gave `render`
a default body because `Core` handles that variant itself. That is a
one-variant special case tied to the view, not a mechanism for defaulting
effects, and this RFC does not extend it.

**Publish shell packages.** Give the capability types a facet namespace, use
facet-generate's `ExternalPackage` to import them from a published SwiftPM,
Maven, npm and NuGet package, and ship the handler in that package with native
tests. Real libraries with their own release cadence, but four registries to
publish to in lockstep with the crate, a breaking move of every capability
type out of the app's namespace, and the bincode runtime shared across
packages. Worth revisiting when shipped handlers are large enough to need
native unit tests; the adapter shape does not preclude it, since a published
package could provide the conformer the property takes.

**A byte-level seam.** Publish a package that bundles its own copy of the
types and exposes `handle(bytes) -> bytes`; type generation emits a default
that serialises across the seam. Avoids the namespace move but keeps the four
registries, and adds a protocol-version check the app can get wrong.

**A shared static instead of a property.** Emit the adapter's methods as
`URLSessionHttpHandler.shared.request(op)` with no property. Simpler to
generate and to read, but configuration means either mutating a global or
re-implementing the delegation, and there is no compile-time signal for a
capability that needs configuring. Rejected because the property is what
makes the Configure tier a single declaration.

**One handler per operation.** Bind each operation to its own shipped type
rather than grouping by capability. Loses the shared state a timer table or a
store needs, and multiplies the properties an app would configure.

**Runtime registration.** A `ShellHandlers.register(HttpHandler.self, …)` the
app calls at startup, consulted by the adapter. Discoverable only by reading
generated code, fails at runtime when missing, and invisible to the compiler.
Rejected.

**Documentation.** Keep the handler files in the book and the examples and
tell people to copy them. This is the status quo, and its failure mode is the
one described under Why: rules re-derived per app, drifting as the protocol
evolves.

## Open questions

1. **A default within the adapter.** Should `HttpHandling` supply
   `httpHandler` itself, defaulting to the shipped implementation, so that
   conforming alone is enough and the property is only written to configure?
   Swift could do it from the shipped source, with an extension on the
   generated protocol; Kotlin and C# would need the default expression back
   in the static, because it has to appear in the interface declaration.
   Uniformity across languages argues for leaving the property required.
2. **Logging.** Shells want to see HTTP traffic and timer activity in their own
   logs. A shipped protocol could carry an optional observer, or the shipped
   implementations could take a logger in their configuration. Left to the
   first implementation.
3. **Which implementations to ship.** Tentatively: `crux_http` everywhere
   (`URLSession`, `HttpURLConnection`, `fetch`, `HttpClient`); `crux_time`
   everywhere; `crux_kv` as `UserDefaults` in Swift, `localStorage` where
   present in TypeScript, and a file-backed store taking a directory on the
   JVM and in C#. Because the property is required, a store that needs a
   directory is no different from one that does not. To be settled per
   capability in the implementing pull requests.
4. **Renamed variants.** The adapter uses the variant's emitted name, so a
   renamed variant still binds. If two variants of one effect carry the same
   operation — two `HttpRequest` variants — both delegate to the same
   method, which is correct but worth a note in the book.

## Next steps

1. Release the facet-generate that carries the companion-file hook; the
   per-operation types stack already depends on it.
2. Add `ShellHandler`, `ShellMethod`, `ShellSource` and
   `CodeGenerator::shell_handler` to `crux_core`, record the operation's
   `TypeId` in `EffectVariantMeta`, and add the plugin with its checks.
3. Ship handlers for `crux_http`, `crux_kv` and `crux_time` in all four
   languages, and migrate the notes and weather shells to them, so CI compiles
   every shipped Swift, Kotlin and TypeScript file. Give counter-http a C#
   shell, or add a C# typegen fixture, so the C# files are compiled too.
4. Write the capability-author chapter: how to declare a shipped handler, the
   rules above, and how the examples test it.
