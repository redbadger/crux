# RFC: Shell handlers shipped with capabilities

```admonish
This RFC has been **accepted** and implemented, it's kept for future reference
as additional context for the design choices. It builds on the
[per-operation types RFC](./per-operation-types.md) and the
[generated `Core` RFC](./generated-core.md). See
[Shipped shell handlers](../part-4/typegen.md#shipped-shell-handlers) in the
type generation chapter and the
[migration guide](../guide/migrate-per-operation-types.md#adopting-the-shipped-handlers)
for the practical version. It is written against the compat release of
per-operation types, but nothing in it depends on the shape of the `Operation`
trait, which the [operation kind traits RFC](./operation-kind-traits.md)
proposes to change in the breaking release: `Operation` stays as the
supertrait, hidden and implemented only by the derive, and
`operation::{Notify, Request, Stream}` gain the payload type under the word
that fits each kind.
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
`ClearTimer` arrives first the timer is cancelled and the late answer is harmless.
The capability author is the one who knows those rules, and today every shell
author re-derives them from the book and the examples.

Which HTTP client performs the request, on the other hand, is the shell's
decision, and some shells have strong reasons to make it themselves. So the
proposal is a reference implementation the shell can adopt, not a default it
inherits.

The capability declares what it ships:

```rust
// Rust: crux_http/src/shell.rs

#[cfg(feature = "facet_typegen")]
pub static EXTENSION: ShellExtension = ShellExtension::new("Http")
    .rust_types(register_types)
    .foreign_types(&["HttpHandler", "IHttpHandler", "URLSessionHttpHandler", /* … */])
    .swift(ShellSource::stdlib(include_str!("../shell/swift/Http.swift")))
    .kotlin(ShellSource::stdlib(include_str!("../shell/kotlin/Http.kt")))
    .typescript(ShellSource::stdlib(include_str!("../shell/typescript/http.ts")))
    .csharp(ShellSource::stdlib(include_str!("../shell/csharp/Http.cs")));
```

The app asks for it where it configures type generation:

```rust
// Rust — shared/src/bin/codegen.rs

let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .shell_extension(&crux_http::shell::EXTENSION)?
    .shell_extension(&crux_time::shell::EXTENSION)?
    .build()?;
```

and its handler uses it:

```swift
// Swift — the weather shell's handler
struct WeatherHandler: EffectHandler {
    let http = URLSessionHttpHandler()
    let time = TaskTimeHandler()

    func http(_ operation: HttpRequest) async -> HttpResult { await http.request(operation) }
    func timeNotifyAfter(_ operation: NotifyAfter) async -> TimerId { await time.notifyAfter(operation) }
    func timeClear(_ operation: ClearTimer) async -> TimerId { await time.clear(operation) }

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

The mechanism is not specific to handlers. Type generation emits a **shell
extension**: foreign source that any crate can ship, together with the types
that source names. Nothing about it changes the generated `EffectHandler`. A
capability's shell handler is the main use, and the one this RFC is about,
but a crate with other shell-side code to deliver would use the same type.

A capability declares one `ShellExtension` per capability, not per operation.
The unit of implementation on the shell side is the capability (an HTTP
client, a key-value store, a timer table), because its operations share
state, and because that is the unit an app would want to configure or replace.
By convention the static is `shell::EXTENSION` in the capability crate.

```rust
// Rust
#[non_exhaustive]
pub struct ShellExtension {
    /// UpperCamelCase. Names the companion file (`Http.swift`).
    pub name: &'static str,
    /// Registers every operation and output the shipped sources name.
    pub rust_types: Option<fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>>,
    /// The top-level names the shipped sources declare, in any language
    /// (`HttpHandler`, `IHttpHandler`, `URLSessionHttpHandler`, …).
    pub foreign_types: &'static [&'static str],
    pub swift: Option<ShellSource>,
    pub kotlin: Option<ShellSource>,
    pub typescript: Option<ShellSource>,
    pub csharp: Option<ShellSource>,
}

impl ShellExtension {
    pub const fn new(name: &'static str) -> Self { /* nothing declared, every language None */ }
    pub const fn rust_types(self, register: fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>) -> Self { /* … */ }
    pub const fn foreign_types(self, names: &'static [&'static str]) -> Self { /* … */ }
    pub const fn swift(self, source: ShellSource) -> Self { /* … */ }
    // kotlin, typescript, csharp likewise
}

#[non_exhaustive]
pub struct ShellSource {
    /// The source text, emitted verbatim into the generated module.
    pub source: &'static str,
    /// Manifest entries the source needs beyond the platform standard
    /// library, in the form facet-generate's `manifest_dependencies` takes.
    /// Empty for a handler that needs nothing.
    pub dependencies: &'static [&'static str],
    /// Swift only: `Target.Dependency` expressions for the generated target,
    /// in the form facet-generate's `target_dependencies` takes, for a
    /// handler whose `dependencies` add a package the target must link.
    pub target_dependencies: &'static [&'static str],
}

impl ShellSource {
    pub const fn stdlib(source: &'static str) -> Self { /* both lists empty */ }
    pub const fn dependencies(self, lines: &'static [&'static str]) -> Self { /* … */ }
    pub const fn target_dependencies(self, lines: &'static [&'static str]) -> Self { /* … */ }
}
```

Both types are `#[non_exhaustive]` and built through `const fn` builders,
which a `static` initialiser can call, so a fifth language or another kind
of dependency can be added later without breaking every capability crate.

`foreign_types` lists the names the sources put into the generated module, so
that type generation can check them without parsing the source. The name of the
extension only names its file, and the protocol a handler declares is listed
like any other name.

That is the whole declaration. The `Operation` derive and trait are
untouched, and the breaking release's reshaping of
`operation::{Notify, Request, Stream}` does not affect it.

`rust_types` is there because a shipped source implements the whole capability,
while an app's `Effect` may carry only some of its operations. The weather
app never sends `KeyExists` or `Now`, so left to itself type generation would
not emit them, and `KeyValue.swift` would not compile. The function the
static names registers every operation and output the sources mention —
`HttpRequest::register_types_facet` and its siblings — so that they are
generated whenever the extension is.

A language a capability does not ship (`kotlin: None`) simply has nothing to
emit on that platform: the app implements the methods as it does today.

The static is gated on `facet_typegen`, as `register_types_facet` already is,
so the source text is compiled into the typegen binary and not into the app's
core. This relies on the capability crate having a feature of that name,
which is the existing convention.

### Registering it

The app names the shell extensions it wants on the `TypeRegistry`, before
it builds, because registering one also registers types:

```rust
// Rust — shared/src/bin/codegen.rs
let typegen = TypeRegistry::new()
    .register_app::<Weather>()?
    .shell_extension(&crux_http::shell::EXTENSION)?
    .shell_extension(&crux_time::shell::EXTENSION)?
    .build()?
    .boltffi(BoltFfi::new().swift("Shared").kotlin());
```

This is the one place the app has to know a shipped handler exists, and it
is where the capability's documentation and the book's typegen chapter point.
It is also what keeps the dependency story simple: an extension's
`dependencies` reach the manifest only when the extension is registered, so
no app pays for a library it did not ask for.

Registration is checked at `build`, so that a clash is an error rather than a
generated package that does not compile:

- The `name` is a file stem that works in every language and on every file
  system: an ASCII letter, then ASCII letters, digits or `_`, and not a name
  Windows reserves for a device (`CON`, `NUL`, `COM1` and so on).
- No two registered extensions have the same `name`, ignoring case, because
  `Http.swift` overwrites `http.swift` on a case-insensitive file system. Two
  crates may ship extensions with the same name, and only one of them can be
  registered. Renaming the file on registration would not help, because the
  names the source declares stay the same.
- The `name` is not the name of a file crux generates beside the module
  (`FfiBridge`), ignoring case. It only names files, so it may match a type
  name.
- No name in `foreign_types` is a reserved name of the generated shell API, a
  name the module imports from the serialization runtime (`Serializer`,
  `Bytes` and so on), the name of a namespace the app's types are generated
  into, a name another registered extension declares, or the name of a type
  registered in the app's root namespace.

A `name` equal to the name of the module's own file, ignoring case, would
overwrite it, and is an error when that language is generated. Kotlin and C#
name that file after the last segment of the package or namespace (an `Http`
extension in a `Crux.Http` namespace), and Swift after the whole package
name in `UpperCamelCase` (a `SharedTypes` extension in a `shared_types`
package).

### What is emitted

For each registered extension with source for the target language, the plugin
emits one companion file in the generated module — Swift
`Sources/App/Http.swift`, Kotlin `com/crux/example/weather/Http.kt`, C#
`Company/Shared/Http.cs` — containing the shipped source verbatim, after the
module header the language needs (the `package` line in Kotlin, the
file-scoped `namespace` in C#). In TypeScript, whose modules are one file, the
source goes in the module footer, after every type (and after `Core`, when it
is in this module), so that everything it names is declared before it runs.

Only the root module, the one named after the package, gets the sources. A
namespace of the app's is a module of its own, and each module's
configuration says which namespace it generates, so the plugin asks it
(`config.namespace`) and needs no package name of its own.

A capability's source declares a protocol, `HttpHandler` for `crux_http`, and
one or more implementations of it, and refers to `HttpRequest` and
`HttpResult` unqualified, as any file in that module would:

```swift
// Swift — shipped source, emitted as Sources/App/Http.swift
public protocol HttpHandler: Sendable {
    func request(_ operation: HttpRequest) async -> HttpResult
}

public final class URLSessionHttpHandler: HttpHandler {
    public init(session: URLSession = .shared) { /* … */ }
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
    private val http = UrlConnectionHttpHandler()
    private val time = CoroutineTimeHandler()

    override suspend fun http(operation: HttpRequest) = http.request(operation)
    override suspend fun timeNotifyAfter(operation: NotifyAfter) = time.notifyAfter(operation)
    override suspend fun timeClear(operation: ClearTimer) = time.clear(operation)

    override suspend fun getLocation(operation: GetLocation) = location.current()
    override suspend fun isLocationEnabled(operation: IsLocationEnabled) = location.enabled()
}
```

```ts
// TypeScript
const http = createFetchHttpHandler();
const time = new TimeoutTimeHandler();

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
private val kv = PreferencesKeyValueHandler(preferences)      // app's own conformer
```

```swift
// Swift
let http = URLSessionHttpHandler(session: pinnedSession)
```

**Replace one method.** Write the method body. The shipped implementation
still serves the capability's other operations.

```swift
// Swift
func kvGet(_ operation: GetValue) async -> ValueResult {
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

The app owns the instance, so lifetime is the app's. Every shipped
implementation is constructed by the app, once, where it constructs its
handler, and none offers a shared instance: an app has one core, and so one
handler, so a shared instance would buy nothing. Where there is a sensible
default, an implementation has an initialiser with no arguments for the plain
path, and it always has one that takes configuration. A file store has no
sensible default, because it needs its directory.
Where the platform has a shared client, the plain initialiser uses it, so
there is still one client per process: `URLSessionHttpHandler()` sends with
`URLSession.shared`, and `new HttpClientHttpHandler()` with one static
`HttpClient`, created on first use, that lives for the process, because
`HttpClient` is meant to be created once and reused. A shell that needs a
configured client passes its own, such as a pinned `URLSession`, or, where
`SocketsHttpHandler` is available (it is not on Blazor WebAssembly), an
`HttpClient` over one with a `PooledConnectionLifetime` so that it sees DNS
changes.

That decides where cookies live. With `URLSessionHttpHandler()` they are in
the process-wide `HTTPCookieStorage.shared`, and with
`new HttpClientHttpHandler()` in the static client, which is process-wide
too. `UrlConnectionHttpHandler()` keeps them in an `InMemoryCookieJar` of
that handler instance's own, so two handlers have two sessions. In
TypeScript, `createFetchHttpHandler()` leaves them to the runtime's `fetch`:
in a browser that is the browser's cookie store, for same-origin requests,
because the handler sets no `credentials`, and under Node there are none.

Shipped Swift protocols are `Sendable` and their methods `nonisolated`, matching
`EffectHandler`; a stateful implementation guards its state with a lock rather
than an actor, because the generated types it returns are not `Sendable` and
could not cross an actor's isolation boundary.
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
- The Kotlin source is JVM, not Android: the generated package is a
  `kotlin("jvm")` library, so `android.*` would not compile. The consequence
  matters more than the rule. Where a platform has something better than the
  lowest common denominator — `DataStore` for a store, Cronet for a transport —
  the shipped handler will not be it, and the capability should say so on the
  type rather than leave an app to find out. That is what the Configure tier is
  for. It also imports
  nothing the generated module's header already imports — Kotlin rejects
  the same simple name imported twice — so `com.novi.serde.Bytes` and
  `kotlinx.coroutines.CoroutineScope` are written in full where they occur.
- The Swift source compiles on Linux as well as on Apple platforms, because
  the crate's own test builds it there. The standard library is not the same
  on both: `URLSession`, `URLRequest` and `HTTPURLResponse` are in
  `FoundationNetworking` under corelibs-foundation, so a source that uses them
  imports it under `#if canImport(FoundationNetworking)`.
- The Swift source uses lock-guarded classes, not actors, for shared state:
  the generated operation and output types are not `Sendable`, so an actor
  could not return them across its isolation boundary.
- The TypeScript source reaches platform globals through `globalThis`
  (`globalThis.URL`, `fetch(url, init)` rather than `new Request(..)`),
  because the generated module declares its own `Request` and shadows the
  DOM's.
- C# method names are PascalCase (`Request`, `Get`, `NotifyAfter`), matching
  the generated `IEffectHandler`.
- Declares a protocol named `<Name>Handler` (`I<Name>Handler` in C#) with one
  method per operation, each taking the operation and returning what the
  generated `EffectHandler` method returns, and at least one implementation
  of it. Exposes configuration through the implementation's initialiser, not
  through globals.
- Lists every name it declares at module scope, in every language, in
  `foreign_types`, so that type generation can reject a clash with the app's
  types and namespaces, the generated shell API, the serialization runtime or
  another extension.
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
find. TypeScript's single-file modules need one more hook, `module_footer`,
which facet-generate calls once per module after every type and every
`after_type` output. `after_type` would not do: it follows one type, and
plugin order only orders the plugins within a hook, so the source would have
to be written after whichever type happens to come last. The footer is the
end of the module by definition. `manifest_dependencies` carries the rare
library.

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

**Registering a handler registers the whole capability.** Every operation and
output the sources name is generated, including ones the app never sends, and
all of them land in the app's root namespace. That is more surface for a name
to collide in: the weather example's own `secret::Delete` met `crux_kv`'s
`Delete` this way. The registry used to keep one of them without a word
([#601](https://github.com/redbadger/crux/issues/601)); it now rejects the
pair at registration, naming both Rust types. Renaming the app's type is the
fix, as it is for `Set`; a facet namespace for capability types would be the
structural one.

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
the implementation, so the book's Part II shows handlers whose
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
   everywhere; `crux_kv` as an in-memory store only, in every language. The
   right durable store differs by platform (`DataStore` on Android,
   `UserDefaults` or files on Apple platforms, IndexedDB or `localStorage` in
   a browser). A shipped source can use only its language's standard library,
   so it cannot use `DataStore`. A durable store would also settle what
   `crux_kv` promises (durability, and access from more than one process or
   tab) before [#641](https://github.com/redbadger/crux/issues/641) decides
   it. An app that needs a durable store would implement `KeyValueHandler`
   over the store of its platform, and the weather and notes examples would
   show one for each platform.
3. **Checking the source against the protocol.** Type generation does not
   parse the shipped source, so a protocol method whose shape drifts from its
   operation is caught only when an example shell compiles. Whether a typegen
   fixture that compiles every shipped file for every language is worth
   building, rather than relying on the examples, is a question for the
   implementation.

## Next steps

1. Release the facet-generate that carries the companion-file hook; the
   per-operation types stack already depends on it.
2. Add `ShellExtension`, `ShellSource` and `TypeRegistry::shell_extension` to
   `crux_core`, with the plugin and its checks.
3. Ship handlers for `crux_http`, `crux_kv` (in-memory only) and `crux_time`
   in all four languages, and migrate the notes and weather shells to them, so CI compiles
   every shipped Swift, Kotlin and TypeScript file. Give counter-http a C#
   shell, or add a C# typegen fixture, so the C# files are compiled too.
4. Write the capability-author chapter: how to declare a shipped handler, the
   rules above, and how the examples test it.
