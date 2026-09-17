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
generation to emit that source into its generated package. The shipped source
is an ordinary type in the app's own module, implementing a protocol the
capability defines. The app's `EffectHandler` uses it the way it would use
any other type: it holds an instance and calls it. An app that does not ask
for it gets exactly what it gets today.

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
// Rust — crux_http/src/lib.rs

#[cfg(feature = "facet_typegen")]
pub static HTTP: ShellHandler = ShellHandler {
    name: "Http",
    swift: Some(ShellSource::stdlib(include_str!("../shell/swift/Http.swift"))),
    kotlin: Some(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt"))),
    typescript: Some(ShellSource::stdlib(include_str!("../shell/typescript/http.ts"))),
    csharp: Some(ShellSource::stdlib(include_str!("../shell/csharp/Http.cs"))),
};
```

The app asks for it where it configures type generation:

```rust
// Rust — shared/src/bin/codegen.rs

let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .build()?
    .shell_handler(&crux_http::HTTP)
    .shell_handler(&crux_time::TIME);
```

and its handler uses it:

```swift
// Swift — the weather shell's handler
struct WeatherHandler: EffectHandler {
    let http = URLSessionHttpHandler.shared
    let time = TaskTimeHandler()

    func http(_ operation: HttpRequest) async -> HttpResult { await http.request(operation) }
    func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId { await time.notifyAfter(operation) }
    func timeClear(_ operation: Clear) async -> TimerId { await time.clear(operation) }

    func getLocation(_ operation: GetLocation) async -> Location { /* … */ }
    func isLocationEnabled(_ operation: IsLocationEnabled) async -> Bool { /* … */ }
    func secretFetch(_ operation: Fetch) async -> SecretFetchResponse { /* … */ }
}
```

The three delegating lines are the whole of what the shell writes for
`crux_http` and `crux_time`. The rules live in `Http.swift` and `Time.swift`,
generated into the package, versioned with the crates.

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
shell did not ask for. So:

- **Emission** is opt-in per capability, in the app's type generation
  configuration. An app that does not register a shipped handler gets no
  source and no manifest dependency for it.
- **Wiring** is the shell's own code. The generated `EffectHandler` does not
  change, and nothing is generated that calls the shipped implementation. The
  app's handler holds an instance and delegates to it, one line per
  operation, and those lines are where a reader sees what the app does for
  HTTP or timers.

A shell that drives `EffectDispatcher` itself, or matches on `Effect` by hand,
can register a shipped handler too: it is a plain type, and the shell calls
it from wherever it handles the operation.

### The default has to be easy to leave and easy to return to

A shipped implementation that is hard to configure is worse than none. Real
shells configure their HTTP client (certificate pinning, a shared
`URLSession`, an `OkHttpClient` with interceptors), put their key-value store
somewhere specific, and sometimes replace a whole capability's implementation
with one backed by a platform library. Each of those has to be one
declaration away from the plain path.

## Goals

- A capability crate can ship shell handlers for Swift, Kotlin, TypeScript
  and C#, versioned with the crate, so that regenerating types also brings the
  matching handler.
- The core makes no decision about the shell. What is emitted is chosen by
  the app, nothing is emitted for a capability the app did not ask for, and
  nothing generated calls the shipped code.
- An app that adopts a shipped handler writes, per capability, one
  registration in type generation, one property, and one delegating line per
  operation. It writes none of the protocol rules.
- Configuring a shipped handler, replacing one of its methods, and replacing
  it wholesale are each ordinary code in the app's handler type.
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
- Generating the wiring between `EffectHandler` and the shipped
  implementation. Considered and rejected; see Alternatives.
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
// Rust
pub struct ShellHandler {
    /// UpperCamelCase. Names the companion file (`Http.swift`) and the
    /// protocol the source declares (`HttpHandler`, `IHttpHandler` in C#).
    pub name: &'static str,
    pub swift: Option<ShellSource>,
    pub kotlin: Option<ShellSource>,
    pub typescript: Option<ShellSource>,
    pub csharp: Option<ShellSource>,
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

That is the whole declaration. The operation types are not mentioned, the
`Operation` derive and trait are untouched, and the breaking release's move
to `Notify`, `Request` and `Stream` does not affect it.

A language a capability does not ship (`kotlin: None`) simply has nothing to
emit on that platform: the app implements the methods as it does today.

The static is gated on `facet_typegen`, as `register_types_facet` already is,
so the source text is compiled into the typegen binary and not into the app's
core. This relies on the capability crate having a feature of that name,
which is the existing convention.

### Registering it

The app names the shipped handlers it wants on the built `CodeGenerator`,
next to `boltffi`:

```rust
// Rust — shared/src/bin/codegen.rs
let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .build()?
    .boltffi(BoltFfi::new().swift("Shared").kotlin())
    .shell_handler(&crux_http::HTTP)
    .shell_handler(&crux_time::TIME);
```

This is the one place the app has to know a shipped handler exists, and it
is where the capability's documentation and the book's typegen chapter point.
It is also what keeps the dependency story simple: a handler's
`dependencies` reach the manifest only when the handler is registered, so no
app pays for a library it did not ask for.

Registration is checked: two handlers with the same `name`, or a `name` that
collides with a registered type or a reserved name, is an error.

### What is emitted

For each registered handler with source for the target language, the plugin
emits one companion file in the generated module — Swift
`Sources/App/Http.swift`, Kotlin `com/crux/example/weather/Http.kt`, C#
`Company/Shared/Http.cs` — containing the shipped source verbatim, after the
module header the language needs (the `package` line in Kotlin, the
file-scoped `namespace` in C#). In TypeScript, whose modules are one file, the
source is appended to the module after the types, the way `Core` is today.

The source declares a protocol named after the handler and one or more
implementations of it, and refers to `HttpRequest` and `HttpResult`
unqualified, as any file in that module would:

```swift
// Swift — shipped source, emitted as Sources/App/Http.swift
public protocol HttpHandler: Sendable {
    func request(_ operation: HttpRequest) async -> HttpResult
}

public final class URLSessionHttpHandler: HttpHandler {
    public static let shared = URLSessionHttpHandler(session: .shared)
    public init(session: URLSession) { /* … */ }
    public func request(_ operation: HttpRequest) async -> HttpResult { /* … */ }
}
```

Each protocol method takes the operation and returns what the generated
`EffectHandler` method for that operation returns — the output for a request,
nothing for a notification, and an `EffectSink` for a stream — so the app's
delegation is one expression.

Nothing else is emitted. The generated `EffectHandler`, `EffectDispatcher`
and `Core` are exactly as the earlier RFCs describe them.

### The developer experience

**Use.** Hold an instance of the shipped implementation and delegate.

```kotlin
// Kotlin
class WeatherHandler(private val location: LocationClient) : EffectHandler {
    private val http = UrlConnectionHttpHandler
    private val time = TaskTimeHandler()

    override suspend fun http(operation: HttpRequest) = http.request(operation)
    override suspend fun timeNotifyAfter(operation: NotifyAfter) = time.notifyAfter(operation)
    override suspend fun timeClear(operation: Clear) = time.clear(operation)

    override suspend fun getLocation(operation: GetLocation) = location.current()
    override suspend fun isLocationEnabled(operation: IsLocationEnabled) = location.enabled()
}
```

```ts
// TypeScript
const http = fetchHttpHandler;
const time = new TaskTimeHandler();

const handler: EffectHandler = {
  http: (operation) => http.request(operation),
  timeNotifyAfter: (operation) => time.notifyAfter(operation),
  timeClear: (operation) => time.clear(operation),
  getLocation: async (operation) => location.current(),
  isLocationEnabled: async (operation) => location.enabled(),
};
```

**Configure.** Construct the shipped implementation with what it needs, or
use any other type conforming to the shipped protocol.

```kotlin
// Kotlin
private val http = OkHttpHttpHandler(client)                  // app's own conformer
private val kv = FileKeyValueHandler(context.filesDir)
```

```swift
// Swift
let http = URLSessionHttpHandler(session: pinnedSession)
```

**Replace one method.** Write the method body. The shipped implementation
still serves the capability's other operations.

```swift
// Swift
func kvGet(_ operation: Get) async -> ValueResult {
    if operation.key == "session" { return await keychainGet() }
    return await kv.get(operation)
}
```

**Replace the capability.** Supply a conformer of the shipped protocol, as
under Configure, or write the methods without it. The protocol is also the
natural seam for a fake in tests.

**Do not use it.** Do not register it. Nothing is emitted, and the generated
package is exactly what the per-operation types RFC generates.

When a capability gains an operation, the app's handler stops compiling until
the app adds the delegating line for it. That is the moment to read what the
new operation does, and it is a better moment than finding out from a
regenerated file that does it silently.

### State and concurrency

The app owns the instance, so lifetime is the app's. Shipped implementations
that are safe to share offer a shared instance
(`URLSessionHttpHandler.shared`, a Kotlin `object`, a C# static) for the
plain path, and an initialiser for the configured one. Shipped
implementations that hold state, like a timer table, are constructed by the
app, once, where it constructs its handler.

Shipped Swift protocols are `Sendable` and their methods `nonisolated`, matching
`EffectHandler`; a stateful implementation guards its state with an actor or
a lock, as the weather shell's timer table does today with `@MainActor`.
Kotlin implementations are `suspend` and choose their own dispatcher, as the
weather shell's `withContext(Dispatchers.IO)` does.

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
- Declares a protocol named `<Name>Handler` (`I<Name>Handler` in C#) with one
  method per operation, each taking the operation and returning what the
  generated `EffectHandler` method returns, and at least one implementation
  of it. Exposes configuration through the implementation's initialiser, not
  through globals.
- Declares nothing else at module scope under a name a generated type could
  take. `<Name>Handler` joins the reserved names.
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
find. TypeScript's single-file modules use `after_type`, as `Core` does, and
`manifest_dependencies` carries the rare library.

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

**The app still writes a line per operation.** A shell using `crux_http`,
`crux_kv` and `crux_time` writes a property per capability and a delegating
line per operation, where a generated default would have cost nothing. Those
lines are deliberate — they are where the shell's choice is visible, and
where a new operation surfaces — but they are lines, and a capability with
many operations has many of them.

**The app has to know to register it.** Type generation cannot suggest a
shipped handler, because it does not know which crates ship one until they
are registered. The capability's documentation and the book's typegen chapter
are where an author learns the line exists.

**Stdlib-only implementations are sometimes the second-best implementation.**
The weather Android shell uses OkHttp today and would keep using it, through
the Configure tier, rather than the shipped `HttpURLConnection` handler. The
shipped implementation exists to make the first build work and to carry the
protocol rules, not to be the last word on the client.

## Migration

Nothing is required. A shell that implements `http`, `kvGet` and the rest keeps
compiling and keeps its behaviour. A shell that registers nothing generates
what it generates today.

To adopt, register the handler in type generation, regenerate, replace the
method bodies with delegations to an instance of the shipped implementation,
and delete the file that held them. The notes and weather examples do this in
the implementing pull request, so the book's Part II shows handlers whose
only substance is the app-defined operations.

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
one platform made the property required there and optional elsewhere.

**A generated adapter.** The second draft kept the wiring but made it opt-in:
the static listed its operations and methods, `EffectVariantMeta` recorded
each operation's `TypeId`, and type generation emitted a per-capability
`HttpHandling: EffectHandler` protocol with a required `httpHandler` property
and default bodies delegating to it, which the app adopted by conforming.
Counted out, it saved the weather shell almost nothing — five delegating lines
became three conformances and three properties — and cost a method listing
in every capability, `TypeId` plumbing, registration checks and four adapter
emitters. Review asked whether that was making our life difficult for not
much gain, and it was. The delegating lines also turn out to be the better
behaviour: a new operation asks for a line instead of being wired silently.

**Discovery through the operation.** Instead of the app registering the
handler, put `shell = HTTP` on each `#[operation]` so the derive registers it
and every shipped handler is emitted automatically. Zero configuration, and
the foreign compilers strip unused types, but manifest dependencies are not
stripped, so a capability that ships a library would land it in every app.
It also puts a shell-side concern on every operation in the core crate, and
grows the derive for a feature the trait does not know about. The one line in
the codegen binary is the price of keeping both decisions in the shell.

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
native unit tests; nothing here precludes it, since a published package could
provide the conformer the app's property holds.

**A byte-level seam.** Publish a package that bundles its own copy of the
types and exposes `handle(bytes) -> bytes`; type generation emits a default
that serialises across the seam. Avoids the namespace move but keeps the four
registries, and adds a protocol-version check the app can get wrong.

**One handler per operation.** Ship each operation's implementation as its own
type rather than grouping by capability. Loses the shared state a timer table
or a store needs, and multiplies the instances an app would hold.

**Documentation.** Keep the handler files in the book and the examples and
tell people to copy them. This is the status quo, and its failure mode is the
one described under Why: rules re-derived per app, drifting as the protocol
evolves.

## Open questions

1. **Logging.** Shells want to see HTTP traffic and timer activity in their own
   logs. A shipped protocol could carry an optional observer, or the shipped
   implementations could take a logger in their configuration. Left to the
   first implementation.
2. **Which implementations to ship.** Tentatively: `crux_http` everywhere
   (`URLSession`, `HttpURLConnection`, `fetch`, `HttpClient`); `crux_time`
   everywhere; `crux_kv` as `UserDefaults` in Swift, `localStorage` where
   present in TypeScript, and a file-backed store taking a directory on the
   JVM and in C#. To be settled per capability in the implementing pull
   requests.
3. **Checking the source against the protocol.** Type generation does not
   parse the shipped source, so a protocol method whose shape drifts from its
   operation is caught only when an example shell compiles. Whether a typegen
   fixture that compiles every shipped file for every language is worth
   building, rather than relying on the examples, is a question for the
   implementation.

## Next steps

1. Release the facet-generate that carries the companion-file hook; the
   per-operation types stack already depends on it.
2. Add `ShellHandler`, `ShellSource` and `CodeGenerator::shell_handler` to
   `crux_core`, with the plugin and its name check.
3. Ship handlers for `crux_http`, `crux_kv` and `crux_time` in all four
   languages, and migrate the notes and weather shells to them, so CI compiles
   every shipped Swift, Kotlin and TypeScript file. Give counter-http a C#
   shell, or add a C# typegen fixture, so the C# files are compiled too.
4. Write the capability-author chapter: how to declare a shipped handler, the
   rules above, and how the examples test it.
