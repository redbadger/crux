# iOS/macOS

This is the first of the shell chapters. We'll walk through how the Swift side talks to the Rust core, how each effect gets carried out, and how the views consume the view model. The other shell chapters follow the same structure in their own idioms.

## The WeatherKit package

The Apple shell is split into two Swift targets:

- **`WeatherApp`** (the app target) — just a few files: the `@main` struct that builds the `Core`, and `ContentView` as the root view.
- **`WeatherKit`** (a local Swift Package) — everything else: the `WeatherHandler`, every effect handler, every screen, and the preview helpers.

The split exists because building Swift is much faster than rebuilding the whole Rust framework, and SPM gives you the kind of iteration loop you'd expect from `cargo`. When you're tweaking a view, you only recompile the package. When you're iterating on effect handlers, same — the Rust library (and the Swift bindings it emits) only recompile when the core changes.

WeatherKit never touches the Rust FFI. Neither, in fact, does any code we wrote: the generated `App` package contains the one type that does, and the app target constructs a `Core` from it. That's what lets SwiftUI previews run without the Rust framework loaded. More on that at the end.

## Booting the Core

Here's the app entry point:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/WeatherApp.swift:start}}
```

Build the generated `Core` from a `WeatherHandler`, keep it in `@State`, wire up an `updater`, and send `Event::Start` to kick the lifecycle. After that, the core starts fetching the API key and favourites — everything we described in chapter 3.

`Core` comes from the generated `App` module, hence the `import App`. `struct WeatherApp: App` still resolves to SwiftUI's protocol — Swift looks for a protocol in that position, not a module — so the two names don't clash.

## The FFI bridge

`Core(handler:)` is a convenience: underneath, `Core` talks to Rust through a `CoreBridge` protocol with three byte-level methods — `update` and `resolve` return the serialized requests the core produced, `view` the serialized view model — and the generated package implements it over BoltFFI's `CoreFfi` in a file of its own, `FfiBridge.swift`:

```swift
public struct FfiBridge: CoreBridge, @unchecked Sendable {
    private let ffi = Shared.CoreFfi()

    public func update(_ event: [UInt8]) -> [UInt8] {
        [UInt8](ffi.update(data: Data(event)))
    }

    public func resolve(_ id: UInt32, _ output: [UInt8]) -> [UInt8] {
        [UInt8](ffi.resolve(id: id, data: Data(output)))
    }

    public func view() -> [UInt8] {
        [UInt8](ffi.view())
    }
}
```

Nothing here knows about bincode or about Swift types — the generated `Core` does the serializing — so this is the only place that knows `CoreFfi` exists, and it is generated because the codegen was told where BoltFFI put it:

```rust,ignore
.boltffi(BoltFfi::new().swift("Shared") /* … */)
```

That line is also why the generated `App` package depends on the `Shared` package BoltFFI produces, and declares the same `platforms:` floor. The app target links `App` and gets `Shared` through it.

One annotation is worth a look. `CoreBridge` is `Sendable` while `CoreFfi` is a class Swift can't prove safe, so the conformance is `@unchecked Sendable` — sound, because the Rust `Bridge` behind the handle guards its state with mutexes. If you ever write a `CoreBridge` of your own in a target that defaults to `MainActor` isolation, it also needs to be `nonisolated`, because the protocol's requirements are not actor-isolated. The preview bridge at the end of this chapter is one.

## Handling effects

The loop — serialize the event, call the bridge, deserialize the requests, dispatch each one, resolve, go round again — is the generated `Core`. Its whole public surface is:

```swift
@available(macOS 14.0, iOS 17.0, tvOS 17.0, watchOS 10.0, *)
@Observable @MainActor public final class Core {
    public private(set) var view: ViewModel
    public init(bridge: any CoreBridge, handler: any EffectHandler)
    public convenience init(handler: any EffectHandler)
    public func update(_ event: Event)
    public func process(_ requests: [Request])
    public func process(bytes: [UInt8])
}
```

`Core` handles `Render` itself: when one arrives it re-reads the view from the bridge and stores it in `view`, which is the class's one observable property. Everything else goes to the generated `EffectDispatcher`, which calls the matching `EffectHandler` method and resolves the request afterwards — never for a notification, once for a request. When a request is resolved, `Core` passes the bytes back through the bridge and loops over any **new** requests that come back. This is a direct consequence of `Command`'s async nature: a command written with `.await` points produces its next effect only after the previous one is resolved. The shell has to keep processing until the command's task finishes — and now nothing in the shell has to remember to.

What the shell writes is the `EffectHandler`. In WeatherKit that is `WeatherHandler`:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/WeatherHandler.swift}}
```

`render` has a generated default, because `Core` owns it. The `crux_http`, `crux_kv` and `crux_time` methods are one line each: they delegate to the handlers those crates ship, which the codegen binary registers and type generation writes into the `App` package as `Http.swift`, `KeyValue.swift` and `Time.swift`. `WeatherHandler` holds one instance of each — the shared `URLSessionHttpHandler`, a `UserDefaultsKeyValueHandler` over a suite of the app's own, and a `TaskTimeHandler`, which owns the timer table. The app's own operations, location and secret, live beside the platform code they use in `location.swift` and `secret.swift`, as extensions on `WeatherHandler`.

Note the `nonisolated`. The generated `EffectHandler` is `Sendable` and its requirements are not actor-isolated, but `WeatherHandler` is `@MainActor` — so the handler methods are `nonisolated` and hop to the main actor only where they touch main-actor state. URLSession, Keychain and CoreLocation work doesn't belong on the main actor anyway.

`http(_:)` is `async` and returns an `HttpResult`. That's the whole contract — the operation declares that it is answered exactly once, with an `HttpResult`, so the method signature says so and the dispatcher does the resolving. There's no request id in sight and no `resolve` call to get wrong.

What produces the `HttpResult` is `URLSessionHttpHandler`, in the generated `Http.swift`: it turns the `HttpRequest` into a `URLRequest`, maps the response back, and knows that a `URLError.timedOut` is `HttpError.timeout`, a bad URL is `HttpError.url`, and everything else `URLError` throws is `HttpError.io`. Those rules belong to `crux_http`, so `crux_http` ships them, and a shell that needs a pinned or otherwise configured session writes `URLSessionHttpHandler(session:)` in place of `.shared`. A shell with its own HTTP stack conforms its own type to the `HttpHandler` protocol instead. See [Shipped shell handlers](../../part-4/typegen.md#shipped-shell-handlers).

The timer handler is worth a glance for the same reason: `TaskTimeHandler.notifyAfter` waits out the duration and returns the `TimerId`, and `clear` cancels the pending timer and returns the same `TimerId`, which is the core's cue that the timer is gone. If the timer fires anyway before the clear reaches the shell, the late answer to `timeNotifyAfter` is ignored: the core stopped waiting for it when the timer was cleared. That rule used to live in a comment in this shell; now it lives in `Time.swift`, next to the code that follows it, in every app that uses `crux_time`.

## Views driven by the ViewModel

The generated `Core` is `@Observable`, so `WeatherApp` puts it straight into the SwiftUI environment and views read it with `@Environment(Core.self)`. There is no shell-side box in between. Every `Render` replaces `view` wholesale, so every view that reads `core.view` is invalidated and SwiftUI diffs the resulting view tree; that is cheap, and it is the same granularity a callback into a store would give. Finer-grained invalidation — only the screen whose slice changed — would need the view model itself to be observable, which is where diff-based view updates would come in later.

The root `ContentView` dispatches on the top-level `ViewModel` variants:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/ContentView.swift}}
```

Four lifecycle states, four views. `ActiveView` in turn dispatches on the active sub-variants (Home vs Favorites), and so on down the tree — each level of the model has a corresponding layer of view.

When the user taps a button, the view sends an event via the `CoreUpdater` that was injected into the environment at the app root. The event travels through the bridge, the core updates its state, and the `@Observable` property re-renders the view.

## Previewing without the core

Because WeatherKit never touches the FFI, previews don't need the Rust framework. They do need a `Core`, since that is what the views read from, so `PreviewCore.swift` provides the two things a real `Core` is built from: a `CoreBridge` that answers `view()` with a fixed view model and returns no requests, and an `EffectHandler` whose methods never run, because a preview never sends an event. With those and a `CoreUpdater.forPreview()` that swallows events, a preview builds a genuine generated `Core` and injects it:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Preview/PreviewCore.swift}}
```

Previews run as fast as regular SwiftUI previews, no FFI boundary to cross.

## What's next

That's one shell end-to-end. The core doesn't know or care what platform it's on; everything platform-specific lives here. The other shell chapters walk through the same story — booting the core, the bridge, the effect handlers, the views — in Kotlin, Rust with Leptos, and TypeScript with React.

Happy building!
