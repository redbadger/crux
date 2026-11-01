# RFC: Shell protocols shipped with capabilities

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and the
[generated `Core` RFC](./generated-core.md). Nothing in it depends on the
`Operation` trait, which the breaking release of per-operation types replaces
with `Notify`, `Request` and `Stream`.
```

This RFC proposes that each capability crate ships the shell side of its
contract as a **protocol**: the handler interface that a shell implements for
the capability, in Swift, Kotlin, TypeScript and C#, with the rules that every
implementation must follow. Type generation would emit the protocol for every
app that uses an operation of the capability, with no registration. The
protocol is an interface only. Each app writes its own implementation, in each
of its shells.

## Summary

After the per-operation types RFC, a shell implements the generated
`EffectHandler`: one method for each effect variant, each with a typed
operation in and a typed output out. After the generated `Core` RFC, that is
all a shell writes. For the operations of a published capability (`crux_http`,
`crux_kv`, `crux_time`), much of what the shell writes follows rules that
belong to the capability, not to the app. For example, a 4xx or 5xx status is
an answer and not an error, a `URLError.timedOut` becomes `HttpError.timeout`,
and a `NotifyAfter` whose timer is cleared can still answer, harmlessly. The
capability author knows those rules. Today every shell author derives them
again from the book and the examples.

The capability would declare its protocol once, with one source file for each
language and the top-level names that each file declares:

```rust
// Rust: crux_kv/src/shell.rs

crux_core::__crux_core_facet_typegen_items! {
    pub static PROTOCOL: ShellProtocol = ShellProtocol::new("KeyValue")
        .rust_types(register_types)
        .swift(include_str!("../shell/swift/KeyValue.swift"), &["KeyValueHandler"])
        .kotlin(include_str!("../shell/kotlin/KeyValue.kt"), &["KeyValueHandler"])
        .typescript(include_str!("../shell/typescript/keyValue.ts"), &["KeyValueHandler"])
        .csharp(include_str!("../shell/csharp/KeyValue.cs"), &["IKeyValueHandler"]);
}
```

Each operation would name the protocol:

```rust
#[derive(Operation, Facet, Clone, Debug, Serialize, Deserialize)]
#[operation(request, output = ValueResult, protocol = crate::shell::PROTOCOL)]
pub struct GetValue {
    pub key: String,
}
```

An app that uses `GetValue` would then get `KeyValueHandler` in its generated
package, with the rules in its doc comment:

```swift
// Swift: emitted as Sources/App/KeyValue.swift
public protocol KeyValueHandler: Sendable {
    func get(_ operation: GetValue) async -> ValueResult
    func set(_ operation: SetValue) async -> ValueResult
    // one method for each operation of crux_kv
}
```

The app implements the protocol over the store of its choice, and its
`EffectHandler` delegates to that implementation, one line for each
operation:

```swift
// Swift: the weather shell's handler
struct WeatherHandler: EffectHandler {
    let kv: any KeyValueHandler

    func kvGet(_ operation: GetValue) async -> ValueResult { await kv.get(operation) }
    func kvSet(_ operation: SetValue) async -> ValueResult { await kv.set(operation) }
    // the other operations
}
```

## Why?

### The rules belong to the capability

The weather example's Swift shell, after it adopts the generated `Core`, has
193 lines of handler code. Of those, 153 implement `crux_http`, `crux_kv` and
`crux_time`: `http.swift` (52), `keyValue.swift` (44) and `time.swift` (57).
The Kotlin shell has the same 175 lines in `HttpHandler.kt`,
`KeyValueHandler.kt` and `TimeHandler.kt`. The notes example's TypeScript
shell has its own key-value and timer handlers, with the same doc comment
about a cleared timer.

Two different things are in those files. One is the platform call:
`URLSession.data(for:)`, OkHttp, `fetch`. The other is the contract: how a
response and its errors map onto `HttpResult`, what a timer answers with and
when, what a cleared timer does. The platform call is the business of the
shell. The contract is the business of the capability author, and the author
would write it once, next to the Rust that defines it, and ship it. The rule
about the late timer answer was settled in the
[review](./per-operation-types.md) of the per-operation types RFC, and today it
is in a doc comment in the weather shell's `time.swift`. That is the wrong
place for it.

A protocol holds the contract in a form that each language checks: a method
for each operation, with the right types, and the rules in its doc comment. A
shell that implements the protocol cannot forget an operation, and it reads
the rules where it writes the code.

### The implementation belongs to the app

An implementation belongs to each app and shell. Real shells configure their
HTTP client (certificate pinning, a shared `URLSession`, an `OkHttpClient` with
interceptors), put their key-value store in a specific place, and use platform
libraries that the standard library does not have. One implementation that
suits every app is hard to make. Thus the capability crates would ship the
protocol, and leave the implementation to the app. "Alternatives considered"
describes the design that also shipped implementations, and why this RFC does
not propose it.

## Goals

- Each capability crate ships its protocol in Swift, Kotlin, TypeScript and
  C#, versioned with the crate, so that a new version of the crate brings the
  matching protocol when the app generates its types again.
- An app gets the protocol of each capability that it uses, with no
  registration and no feature of the capability crate.
- The protocol uses only the standard library of each language, so it adds
  nothing to the manifest of an app.
- Type generation stays independent of package registries and of BoltFFI. The
  protocol compiles against the app's own generated types, in the same module.
- Nothing changes for a shell that keeps its own handler methods. It can
  implement the protocol when it is ready.

## Non-goals

- Shipping implementations of the protocols. See "Alternatives considered".
- Publishing shell packages to SwiftPM, Maven, npm or NuGet. That is a
  different mechanism with a different cost model, discussed under
  "Alternatives considered".
- Protocols for an app's own operations. The mechanism is available to any
  crate that declares operations, but its purpose is published capabilities.
- Generating the wiring between `EffectHandler` and the app's implementation
  of a protocol.
- The Rust side of the FFI, as in the generated `Core` RFC.

## Design

### Declaring a protocol

A capability declares one `ShellProtocol`, not one for each operation. The
operations of a capability share state on the shell side (an HTTP client, a
key-value store, a timer table), so a shell implements them together. By
convention, the static is `shell::PROTOCOL` in the capability crate.

```rust
// Rust
#[non_exhaustive]
pub struct ShellProtocol {
    /// UpperCamelCase. Names the companion file (`Http.swift`).
    pub name: &'static str,
    /// Registers every operation and output that the sources name.
    pub rust_types: Option<fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>>,
    pub swift: Option<ProtocolSource>,
    pub kotlin: Option<ProtocolSource>,
    pub typescript: Option<ProtocolSource>,
    pub csharp: Option<ProtocolSource>,
}

#[non_exhaustive]
pub struct ProtocolSource {
    /// The source text, emitted unchanged into the generated module.
    pub source: &'static str,
    /// The top-level names that the source declares (`HttpHandler`, or
    /// `IHttpHandler` in C#).
    pub foreign_types: &'static [&'static str],
}

impl ShellProtocol {
    pub const fn new(name: &'static str) -> Self { /* no source in any language */ }
    pub const fn rust_types(self, register: fn(&mut TypeRegistry) -> Result<&mut TypeRegistry, TypeGenError>) -> Self { /* … */ }
    pub const fn swift(self, source: &'static str, foreign_types: &'static [&'static str]) -> Self { /* … */ }
    // kotlin, typescript and csharp in the same way
}
```

Both types are `#[non_exhaustive]` and built with `const fn` builders, which a
`static` initialiser can call. Thus a fifth language can be added later
without a breaking change to every capability crate.

`foreign_types` lists the names that a source puts into the generated module,
so that type generation can check them without a parser for the source. The
names belong to each language, because C# adds an `I` to the name of an
interface.

`rust_types` is necessary because a shell implements the whole protocol, but
an app's `Effect` can carry only some of the operations of a capability. The
weather app never sends `KeyExists` or `Now`, so type generation would not emit
those types, and `KeyValue.swift` would not compile. The function registers
every operation and output that the sources name (`GetValue::register_types_facet`
and the others), so that type generation emits them with the protocol.

A language for which a capability has no source gets nothing. The app writes
the interface itself, as it does today.

### How an app gets the protocol

The app does not register a protocol. `#[derive(Operation)]` would take
`protocol = path`, and the derive would then override `register_types_facet`
to call `TypeRegistry::shell_protocol`. The `#[effect]` macro already calls
`register_types_facet` for every variant, so an app that uses one operation of
a capability gets the protocol of that capability. The `Operation` trait does
not change.

`shell_protocol` would record each protocol once, and run its `rust_types`.
Each operation of the capability calls `shell_protocol` again from its own
`register_types_facet`, and the record stops that recursion. A different
protocol with the same name is recorded too, and the name checks report the
clash.

The protocol would follow the `facet_typegen` feature of `crux_core`, not a
feature of the capability crate. The override, and the protocol items of the
capability crate, would be inside `__crux_core_facet_typegen_items!`. This
hidden macro of `crux_core` emits its input only when the `facet_typegen`
feature of `crux_core` is on, and the compiler evaluates that feature in
`crux_core`. Thus the protocol follows the feature that the app already turns
on for type generation, and the app needs no feature of the capability. The
source text is compiled into the type generation binary and not into the
app's core.

### What is emitted

For each protocol with a source in the target language, type generation would
emit one companion file in the root module (Swift `Sources/App/Http.swift`,
Kotlin `com/crux/example/weather/Http.kt`, C# `Company/Shared/Http.cs`). The
file holds the source unchanged, after the module header that the language
needs (the `package` line in Kotlin, the file-scoped `namespace` in C#). A
TypeScript module is one file, so the protocols go in the module footer,
after every type.

A namespace of its own is a module of its own in every language, so the
protocols go only into the root module, which holds the app's types that have
no namespace.

Each protocol method takes the operation and returns what the generated
`EffectHandler` method for that operation returns: the output for a request,
nothing for a notification, and an `EffectSink` for a stream. Thus the app's
delegation is one expression.

Nothing else is emitted. The generated `EffectHandler`, `EffectDispatcher`
and `Core` are exactly as the earlier RFCs describe them. The protocol is
plain code, so a shell that drives `EffectDispatcher` itself, or matches on
`Effect` by hand, can implement it too.

### Name checks

`TypeRegistry::build` would check every protocol, so that a clash is an error
and not a generated package that does not compile:

- The `name` is a file stem that works in every language and on every file
  system: an ASCII letter, then ASCII letters, digits or `_`, and not a name
  that Windows reserves for a device (`CON`, `NUL`, `COM1` and so on).
- No two protocols have the same `name`, ignoring case, because `Http.swift`
  overwrites `http.swift` on a case-insensitive file system.
- The `name` is not the name of a file that crux generates beside the module
  (`FfiBridge`), ignoring case.
- No name in `foreign_types` is a reserved name of the generated shell API, a
  name that the module imports from the serialization runtime (`Serializer`,
  `Bytes` and so on), the name of a namespace of the app's types, a name that
  another protocol declares, or the name of a type in the root namespace.

A `name` equal to the name of the module's own file, ignoring case, would
overwrite that file. Type generation reports it when it generates that
language. Kotlin and C# name the file after the last segment of the package or
namespace (an `Http` protocol in a `Crux.Http` namespace), and Swift after the
whole package name in `UpperCamelCase`.

### The developer experience

**Implement.** Write a type that conforms to the protocol over the platform
client of the app's choice, and delegate to it from the handler.

```kotlin
// Kotlin
class OkHttpHandler(private val client: OkHttpClient) : HttpHandler {
    override suspend fun request(operation: HttpRequest): HttpResult = /* … */
}

class WeatherHandler(
    private val http: HttpHandler,
    private val location: LocationClient,
) : EffectHandler {
    override suspend fun http(operation: HttpRequest) = http.request(operation)
    override suspend fun getLocation(operation: GetLocation) = location.current()
}
```

```ts
// TypeScript
const http: HttpHandler = { request: (operation) => fetchRequest(operation) };

const handler: EffectHandler = {
  http: (operation) => http.request(operation),
  getLocation: async (operation) => location.current(),
};
```

**Test.** The protocol is also the seam for a fake in tests of the shell.

**Keep the handler methods.** A shell can keep its own `EffectHandler`
methods and not use the protocol. The protocol is still emitted, and the
shell compiles as before.

When a capability gains an operation, the app's implementation of the
protocol stops compiling until the app implements the new method. That is the
moment to read what the new operation does.

### Rules for protocol source

These are rules for capability authors. Type generation does not parse the
source, so the crate's own tests and review check them.

- The source uses only the standard library of its language, and puts its
  imports at the top of the file.
- The source declares a protocol named `<Name>Handler` (`I<Name>Handler` in
  C#), with one method for each operation. Each method takes the operation
  and returns what the generated `EffectHandler` method returns.
- The doc comment of the protocol states the rules that every implementation
  must follow. It does not name an implementation or show how to construct
  one.
- Swift protocols are `Sendable`, to match `EffectHandler`. Kotlin methods
  are `suspend`. C# method names are PascalCase (`Request`, `Get`,
  `NotifyAfter`), to match the generated `IEffectHandler`.
- The source lists every name that it declares at module scope in
  `foreign_types`, so that type generation can reject a clash.
- A test in the capability crate generates the code for a small app and
  compiles it with `swift build`, `kotlinc`, `tsc` and `dotnet build`. A
  second test checks that an operation registers the protocol when only the
  `facet_typegen` feature of `crux_core` is on.

The source is in `shell/<language>/` in the capability crate, beside the Rust
that defines the operations, and the two are reviewed together.

### Changes to facet-generate

None. `EmitterPlugin` has **module companion files**: files that a plugin
writes into the directory of the module (`Sources/<Module>/`, the Kotlin
package directory, the C# namespace directory), with the module header
first. The generated `Core` added the hook for its BoltFFI bridge. The
TypeScript protocols use `module_footer`.

## Drawbacks

**Foreign-language source inside Rust crates.** Four files for each
capability that `cargo test` alone cannot compile. The tests of the
capability crates compile them with the toolchain of each language, and skip
a toolchain that is not installed, unless `CRUX_REQUIRE_SHELL_TOOLCHAINS` is
set. CI must install every toolchain and set that variable.

**Four protocols to keep in agreement.** A change to the operations of a
capability touches Rust, Swift, Kotlin, TypeScript and C# in one pull request.
That is intended, because the author who changes the contract knows what the
shells must do. But it raises the cost of a contribution to a capability.

**Every app gets every type of the capability.** An app that uses one
operation gets all the operations and outputs of the capability in its root
namespace. Thus there are more names that can clash. If an app type has one
of those names, the app must rename its type, and the name checks report the
clash when the registry is built. A facet namespace for the types of a
capability would be the structural fix.

**The app still writes the implementation.** The protocol states the rules,
but each shell writes the code that follows them, and the examples show one
way to do it. Two apps can still get a rule wrong in two different ways. The
protocol makes that less likely, because each rule is in the doc comment of
the method that the app implements.

## Migration

Nothing is required. A shell that implements `http`, `kvGet` and the rest
keeps compiling and keeps its behaviour. The protocols appear in its
generated package, and it can use them when it is ready.

To adopt a protocol, make the existing handler code conform to the protocol,
and delegate to it from the `EffectHandler` methods. If the shell already has
a type with the name of the protocol, rename that type. The notes and weather
examples do this in the implementing pull requests.

## Alternatives considered

**Shipped implementations.** The first version of this RFC proposed that each
capability crate also ship implementations of its protocol: `URLSession`,
`HttpURLConnection`, `fetch` and `HttpClient` for `crux_http`, a timer table
for `crux_time`, and a store for `crux_kv`, in each language. A crate declared
a `ShellExtension` static with one `ShellSource` for each language (the
source, and any manifest and Swift target dependencies). The app registered it
in its type generation binary with `TypeRegistry::shell_extension`, which also
registered every type that the sources named. Type generation then emitted the
source beside the module, and the app's handler held an instance of the
shipped implementation and delegated to it. A later revision keyed the
extension by language and platform, so that Android and the JVM could get
different sources.

This RFC does not propose it. An implementation belongs to each app and shell.
Each app configures its client, chooses where its data goes and uses the
libraries of its platform, so one implementation that suits every app is hard
to make. A shipped implementation that suits no app well is a cost to every
capability author, with four implementations to keep in agreement, platform
variants, and toolchain tests for behaviour as well as for shape. The rules
are the part that every app needs, and the protocol carries them. Thus the
capability crates leave the implementation to the app.

**Registration instead of discovery.** The design with shipped
implementations rejected discovery through the operation, for two reasons.
Manifest dependencies are not stripped, so a capability that shipped a library
would put it into every app. And `shell = ..` on every operation put a shell
concern into the core crate. Neither reason applies to a protocol. A protocol
uses only the standard library, so it has no manifest entries. The
`protocol =` attribute is in the capability crate, and its expansion exists
only with the `facet_typegen` feature of `crux_core`, which an app turns on
for type generation in any case. Thus this RFC proposes discovery through the
operation, and the app registers nothing.

**A default wired by the core.** An early draft had each operation name its
shipped method in the `#[operation]` attribute, so that the generated
`EffectHandler` carried defaulted methods, and an app that wrote nothing got
the shipped implementation. The core then decided what the shell does, and an
app that never wanted the default got it anyway.

**A generated adapter.** A second draft emitted a protocol for each capability
that conformed to `EffectHandler`, with default bodies that delegated to a
property, and the app adopted it by conformance. It saved almost no lines, and
it cost a method list in every capability and four adapter emitters. The
delegating lines are also the better behaviour: a new operation asks for a
line instead of a silent default.

**Publish shell packages.** Give the capability types a facet namespace, use
facet-generate's `ExternalPackage` to import them from a published SwiftPM,
Maven, npm and NuGet package, and ship the protocol in that package. Real
libraries with their own release cadence, but four registries to publish to
at the same time as the crate, a breaking move of every capability type out of the
app's namespace, and the bincode runtime shared across packages. Nothing here
prevents it later.

**One protocol for each operation.** Ship an interface for each operation and
not one for each capability. A shell implements the operations of a
capability together, because they share state, so the shell would hold one
object for each operation and share the state between them by hand.

**Documentation.** Keep the rules in the book and the examples, and tell
people to copy them. This is the status quo. Its problem is the one under
"Why?": each app derives the rules again, and they drift as the capability
changes.

## Open questions

1. **Logging.** Shells want to see HTTP traffic and timer activity in their
   own logs. A protocol could have an optional observer method, or each app
   could log in its own implementation. Left to the apps for now.
2. **Shared test cases.** Each app tests its own implementation. A capability
   crate could also ship test cases that any implementation must pass, in each
   language. Whether that is worth the cost is a question for later.

## Next steps

1. Add `ShellProtocol`, `ProtocolSource`, `TypeRegistry::shell_protocol` and
   the `protocol =` attribute of `#[derive(Operation)]`, with the plugin and
   its name checks.
2. Ship the protocols of `crux_http`, `crux_kv` and `crux_time` in all four
   languages, with a test in each crate that compiles them with each
   toolchain.
3. Make the example shells implement the protocols.
4. Describe the protocols in the book: what an app gets, and how a capability
   author declares one.
