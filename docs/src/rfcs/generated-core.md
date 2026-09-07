# RFC: A generated shell-side `Core`

```admonish
This RFC is **proposed**. It builds on the
[per-operation types RFC](./per-operation-types.md) and is implemented in the
same pull request as this text, on top of that RFC's implementation stack, so
that reviewers can read real code alongside the proposal. If accepted, it would
ship in the same `crux_core` release as per-operation types. The
[type generation chapter](../part-4/typegen.md#the-generated-core) describes the
emitted API, and the shell chapters in Part II show the migrated examples.
```

This RFC proposes that type generation emit the whole shell-side core loop —
serialise an event, cross the FFI, deserialise the requests, hand each to the
effect handler, resolve with what comes back, and repeat — as a `Core` class in
Swift, Kotlin, TypeScript and C#, talking to the Rust core through a small,
generated, byte-level `CoreBridge` protocol that the shell satisfies with a
handful of lines around the BoltFFI bindings.

## Summary

The per-operation types RFC gave type generation two facts it did not have
before: how many times each effect is resolved, and with what type. From those
it emits an `EffectHandler` interface and an `EffectDispatcher`, so the shell
no longer calls `resolve` itself. That removed the part of the shell glue that
was easy to get wrong. What is left is a loop that is the same in every shell,
in every language, in every Crux app:

1. serialise the `Event` with bincode and pass the bytes to the core's `update`;
2. deserialise the `Requests` that come back;
3. for each request, if it is `Render`, re-read `view()` and publish it;
   otherwise hand it to the dispatcher;
4. when the dispatcher resolves a request, pass the bytes to the core's
   `resolve`, and go back to step 2 with what it returns.

Type generation knows every type in that loop. It knows the `Event` and the
`ViewModel` because `register_app` registers them; it knows `Request` and
`Requests` because the effect macro registers them; it knows which variant is
`Render` because the operation type is `crux_core::render::RenderOperation`.
So it can emit the loop, and this RFC has it do so.

## Why?

### The residue after the handler API

Take the weather example's Swift shell as it stood after adopting
`EffectHandler`. Its glue was four files: `core.swift` (72 lines), which held
the loop, the dispatcher and the `render` arm; `bridge.swift` (26), a
hand-written `CoreBridge` protocol plus a `FakeBridge` for previews;
`handler.swift` (23), a `render` implementation whose body was "re-read the
view"; and `LiveBridge.swift` (50), the bincode plumbing around `CoreFfi`. Of
those 171 lines, roughly 140 say nothing about the weather app. They would be
the same, character for character, in any other Crux app's Swift shell, and
they were: the counter and counter-http examples carried their own copies.

The Kotlin shell had one 171-line `Core.kt` doing all of the above, of which
the app-specific part was eleven one-line delegations. Every TypeScript shell
in the repository — six of them — carried a byte-identical `deserializeRequests`
that hand-rolled the length-prefixed loop the generated `Requests` class
already implements.

### Hand-rolled glue gets things wrong quietly

`LiveBridge.swift` guarded its deserialisation with `if bytes.count < 8 {
return [] }`. A bincode `Requests` is never shorter than its eight-byte length
prefix, so the guard never fired, but nothing said so, and a reader had to work
out whether it was load-bearing. The old counter-http Swift shell resolved an
SSE stream in a 42-line block that was duplicated, verbatim, for the terminal
event. None of this is hard; all of it is the sort of thing that is written
once, copied, and not looked at again — which is exactly the category of code a
generator should own.

### `render` points the wrong way

With the handler API, `render` is the one handler method that needs the core:
it has to call `view()` and publish the result. So the app's handler ends up
holding a reference back to whatever owns the bridge, or the handler *is* that
object, as it was in weather. Either way the handler, which is supposed to be
the shell's answer to the core's requests, has to know about the core's view.
The generated `Core` is the natural owner of the view, and once it owns `Render`
the handler is purely outbound again.

### It sets up the next step

Crux is
[exploring](https://github.com/redbadger/difficient) sending view model
*diffs* over the wire rather than whole view models. Applying a diff needs
something on the shell side that holds the current view and can patch it. If
every app hand-writes the loop, every app has to own that mirror and wire up the
patching when diffs arrive. If a generated `Core` holds the view, the diff
plugin changes the inside of one generated method and the app-facing surface
does not move.

## Goals

- Generate the loop once, in the type generation plugins that already emit the
  handler API, for all four languages.
- Keep type generation independent of the FFI binding generator. The
  generated code must not import BoltFFI's output, whose package, module and
  class names differ per language and per project.
- Keep the shell's remaining code purely about the platform: an
  `EffectHandler` for the platform work, and a few lines adapting the FFI
  bindings to a byte-level protocol.
- Leave the door open for previews and tests that never load the Rust library.
- Hold the current `ViewModel` inside the generated code, so diff-based
  updates can land later without changing how apps use it.

## Non-goals

- Generating the Rust side of the FFI (`shared/src/ffi.rs`). It is also
  boilerplate, but BoltFFI 0.29's binding generator reads source without
  expanding macros, so a `crux_core` macro emitting `#[boltffi::export]` would
  be invisible to it. Worth revisiting when the examples move to a BoltFFI that
  reads binding metadata from the built artifact.
- Rust shells. Leptos, Yew and Dioxus call the typed `Core<App>` directly and
  match on the `Effect` enum; there is no serialisation and nothing to generate.
- UI-framework observability. `@Observable`, `StateFlow`, React state and
  `INotifyPropertyChanged` are the shell's business; the generated `Core`
  exposes the view through the cheapest idiom each language has and stops
  there.
- Stream termination, which the per-operation types RFC leaves open. The
  generated `Core` neither helps nor hinders it.

## Design

### The bridge is a protocol over bytes

The generated code needs three things from the Rust core: `update(bytes) ->
bytes`, `resolve(id, bytes) -> bytes` and `view() -> bytes`. Type generation
emits exactly that as a protocol (`CoreBridge` in Swift, Kotlin and
TypeScript, `ICoreBridge` in C#) and nothing else. The shell implements it
around whatever BoltFFI generated — in Swift that is three one-line methods
converting `Data` to `[UInt8]` and back; in Kotlin and TypeScript the bytes
pass straight through — and hands the instance to `Core`.

The protocol is the seam for two other things. A preview or a test can
implement it with canned bytes and never load the Rust library, which is what
the weather example's `FakeBridge` did by hand. And a shell whose Rust side
pushes effects out of band, as the middleware examples do through a
`CruxShell.process_effects(bytes)` callback, can feed those bytes to the same
`Core` through a public `process(bytes)`.

Three closures would have done the same job with slightly less ceremony. A
named protocol was chosen because the fake is a real use, and because a
protocol gives the adapter a name the documentation can point at.

### `Core` owns the loop

`Core` is constructed with a bridge and an `EffectHandler`. On construction it
reads the initial view. `update(event)` serialises the event, calls the bridge,
and processes the requests. Processing walks the requests in order and hands
each to an `EffectDispatcher` it built with a resolve callback that calls the
bridge's `resolve` and processes what comes back — the recursion that a
`Command` with several `.await` points depends on, and that every hand-written
shell had to remember.

`process(requests)` and `process(bytes)` are public. The first exists so that
the shell can drive the loop from requests it obtained itself; the second is
for the middleware callback shape mentioned above, and it tolerates an empty
byte array, which is what those callbacks and the middleware `update`/`resolve`
hand over when there is nothing to do.

### `Core` owns `Render` and holds the view

Type generation records, per effect variant, whether the operation type is
`crux_core::render::RenderOperation`. When it is, `Core` intercepts that
variant before dispatching: it re-reads the view from the bridge, stores it,
and notifies. The handler never sees `Render`, and the generated
`EffectHandler` gives `render` a default that does nothing — a protocol
extension in Swift, a default body in Kotlin and C#, an optional member in
TypeScript — so an app handler simply does not mention it.

The view is *held*, not merely forwarded, for the reason given above under
"It sets up the next step": whoever applies a diff needs the previous value.
Today the held value is replaced wholesale on every `Render`. When a diff
plugin exists it will patch the held value instead, and `update`, `view` and
the notification stay as they are.

Notification uses the cheapest idiom each language has that is not tied to a
UI framework: a callback in Swift (`onView`, `@MainActor`), TypeScript and C#
(`Action<ViewModel>`), and a `StateFlow<ViewModel>` in Kotlin, where a
coroutine scope is already needed for the `suspend` dispatcher and `StateFlow`
is what every Android consumer would wrap the callback in anyway. The callback
is not fired for the initial view; the caller reads `view` if it wants it.

An effect enum with no `RenderOperation` variant gets no `Core`. There is no
view loop to own in that case, and the handler and dispatcher are still
emitted for such an app to drive itself.

### Concurrency, per language

- **Swift**: `Core` is `@MainActor` and `final`. The dispatcher runs request
  handlers in `Task`s and resolves from wherever they finish, so the resolve
  callback hops back with `Task { @MainActor in ... }` before touching the
  bridge or the view. This serialises calls into the Rust bridge on the main
  actor, which is what the hand-written shells did, and is cheap: the Rust
  side does no I/O. `CoreBridge` is `Sendable`; a wrapper around BoltFFI's
  non-`Sendable` `CoreFfi` class declares itself `@unchecked Sendable`, which
  is sound because the Rust `Bridge` guards its state with mutexes. `Core`
  carries the same `@available` as the dispatcher, because the generated
  package declares no platforms; that is also why it is not `@Observable`,
  and why a SwiftUI shell keeps a small observable holder that `onView`
  writes to.
- **Kotlin**: `Core` takes a `CoroutineScope`. Each request is dispatched in
  its own coroutine, so a slow handler never blocks the requests behind it,
  and each resolution is processed in its own coroutine too. `update` itself
  is synchronous. The `MutableStateFlow` behind `view` is atomic and the Rust
  bridge is internally synchronised, so no further locking is needed.
- **TypeScript**: single-threaded; `update` is synchronous, request handlers
  return promises, and the dispatcher resolves in their continuations.
- **C#**: as TypeScript, with `Task`. `onView` may run on a thread-pool thread
  after an asynchronous request completes, so a UI shell marshals to its
  dispatcher inside the callback.

### What type generation needs to know

Two additions to the metadata the plugins already read:

- `EffectVariantMeta` gains `render: bool`, set by comparing the operation's
  `TypeId` with `RenderOperation`'s. The `#[effect(facet_typegen)]` macro
  already records every variant, so nothing changes for users.
- `TypeRegistry::register_app` records an `AppMeta` with the registry names of
  `App::Event` and `App::ViewModel`, which it previously registered without
  remembering which was which. The first `register_app` wins, matching the
  existing rule that the handler API is emitted for the first registered
  effect. `CodeGenerator::app()` exposes it.

The plugin itself (`type_generation::facet::plugins::core`) is a sibling of the
handler plugin and follows it exactly: it acts on `after_type` for the primary
effect, so `CoreBridge` and `Core` land in the same module as `Effect`, after
the handler API. Writing them as a separate file was considered and rejected,
because facet-generate skips plugin runtime files when the serde runtime comes
from an external package, which would make `Core` silently disappear for some
configurations.

### Names and escape hatches

`Core`, `CoreBridge` and `ICoreBridge` join the reserved names;
`TypeRegistry::build` reports an error if a shared type claims one. A shell
that already had its own `Core` type — as every example did — deletes it or
refers to the generated one by module.

`CodeGenerator::without_core()` turns off `CoreBridge` and `Core` and leaves the
handler API; `without_effect_handlers()` turns off both, since `Core` depends on
the dispatcher.

## Drawbacks

**A defaulted `render` is a silent default.** A shell that drives
`EffectDispatcher` itself and forgets `render` now compiles and never repaints,
where before it failed to compile. The default is documented as existing for
`Core`'s benefit, and the recommended path is `Core`, but it is a real loss of
a compile-time check for the other path.

**The Swift `Core` serialises all bridge calls on the main actor.** For the
Rust core this is fine; a shell that wanted to call the bridge from a
background actor would write its own loop, which remains possible.

**One more fixed-name surface.** Three more reserved names and a shape that
apps build against, which the difficient work will need to preserve.

**The generated Kotlin module depends on `kotlinx-coroutines-core`.** It
already emitted `suspend` functions, which need only the standard library;
`StateFlow` and `launch` need the library, so type generation now adds it to
the generated `build.gradle.kts`.

## Migration

A shell that has adopted the handler API is three steps away:

1. Write the bridge adapter: a type conforming to `CoreBridge` whose three
   methods call BoltFFI's `CoreFfi`.
2. Move the `EffectHandler` methods and their state off the hand-written core
   object into a plain handler type, and delete `render`.
3. Delete the hand-written loop and construct the generated `Core` with the
   adapter, the handler, and — in Swift, TypeScript and C# — a callback that
   publishes the view; in Kotlin, collect `core.view`.

The [migration guide](../guide/migrate-per-operation-types.md) walks through
this for each language, and the notes and weather examples show the result.
Shells that have not adopted the handler API can still do so first; everything
here is additive, and a shell that matches on `Effect` by hand keeps working.

## Alternatives considered

**Generate direct calls into the BoltFFI bindings.** Rejected: the generated
types package would have to import the bindings package, whose name is a
project decision in every language, and the middleware examples have a
different FFI shape altogether. The protocol costs a few lines per shell and
keeps the two generators independent.

**Leave `render` to the app.** A `Core` that treats `Render` as an ordinary
notification is simpler to specify, but every app then owns the view, the
handler needs a way back to the core, and the diff-based future lands in app
code rather than generated code. Rejected for the reasons in "Why?".

**Emit `Core` as a runtime file.** Cleaner in the generated tree, but skipped by
facet-generate's installers when the serde runtime is an external package.
Rejected in favour of emitting into the effect's module.

**Generate `ffi.rs` too.** Not possible with the BoltFFI the examples use, as
noted under non-goals.

## Open questions

1. **View publication idiom.** A callback in three languages and `StateFlow`
   in the fourth is a pragmatic choice, not a principled one. Should the Swift
   `Core` grow an `@Observable` form once the generated package can declare
   platforms, or should observability stay entirely in the shell?
2. **Diff-based updates.** When the difficient plugin exists, does the
   notification carry the whole view, the diff, or both? The held view means
   any of the three is possible without moving `update` or `view`.
3. **The middleware callback.** `process(bytes)` accepts what
   `CruxShell.process_effects` hands over, but the shell still writes the
   `CruxShell` conformance and the `Arc<dyn CruxShell>` constructor by hand.
   Should type generation know about that shape too?
4. **Stream termination** is unchanged from the per-operation types RFC's
   open question 5.

## Next steps

1. Land this with the notes and weather shells migrated, so the book's Part II
   shows the generated `Core` in Swift, Kotlin and TypeScript.
2. Move the remaining examples' shells across in the breaking release, when
   their capabilities move to per-operation types.
3. Prototype the difficient plugin against the held view.
