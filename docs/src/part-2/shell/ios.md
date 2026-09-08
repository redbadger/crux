# iOS/macOS

This is the first of the shell chapters. We'll walk through how the Swift side talks to the Rust core, how each effect gets carried out, and how the views consume the view model. The other shell chapters follow the same structure in their own idioms.

## The WeatherKit package

The Apple shell is split into two Swift targets:

- **`WeatherApp`** (the app target) — just a few files: the `@main` struct, the `LiveBridge` that talks to Rust, and `ContentView` as the root view.
- **`WeatherKit`** (a local Swift Package) — everything else: the `WeatherHandler`, the `ViewStore`, every effect handler, every screen.

The split exists because building Swift is much faster than rebuilding the whole Rust framework, and SPM gives you the kind of iteration loop you'd expect from `cargo`. When you're tweaking a view, you only recompile the package. When you're iterating on effect handlers, same — the Rust library (and the Swift bindings it emits) only recompile when the core changes.

WeatherKit never touches the Rust FFI. The one type that does, `LiveBridge`, lives in the app target and implements the generated `CoreBridge` protocol; the generated `Core` does the rest. That's what lets SwiftUI previews run without the Rust framework loaded. More on that at the end.

## Booting the Core

Here's the app entry point:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/WeatherApp.swift:start}}
```

Construct a `ViewStore`, build the generated `Core` from a `LiveBridge`, a `WeatherHandler` and a closure that writes each new view model into the store, wire up an `updater`, and send `Event::Start` to kick the lifecycle. After that, the core starts fetching the API key and favourites — everything we described in chapter 3.

`Core` comes from the generated `App` module, hence the `import App`. `struct WeatherApp: App` still resolves to SwiftUI's protocol — Swift looks for a protocol in that position, not a module — so the two names don't clash.

## The FFI bridge

`LiveBridge` is the generated `CoreBridge` protocol, implemented over BoltFFI's `CoreFfi`:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/LiveBridge.swift}}
```

Three methods, bytes in and bytes out: `update` and `resolve` return the serialized requests the core produced, `view` the serialized view model. Nothing here knows about bincode or about Swift types — the generated `Core` does the serializing — so this is the only place that knows `CoreFfi` exists. Everything else in the Swift code works with Swift types.

Two annotations are worth a look. The app target builds with `MainActor` as its default isolation and `CoreBridge`'s requirements are not actor-isolated, so the struct is `nonisolated`. And `CoreBridge` is `Sendable` while `CoreFfi` is a class Swift can't prove safe, so the conformance is `@unchecked Sendable` — sound, because the Rust `Bridge` behind the handle guards its state with mutexes.

## Handling effects

The loop — serialize the event, call the bridge, deserialize the requests, dispatch each one, resolve, go round again — is the generated `Core`. Its whole public surface is:

```swift
@MainActor public final class Core {
    public private(set) var view: ViewModel
    public init(bridge: any CoreBridge, handler: any EffectHandler,
                onView: @escaping @MainActor (ViewModel) -> Void)
    public func update(_ event: Event)
    public func process(_ requests: [Request])
    public func process(bytes: [UInt8])
}
```

`Core` handles `Render` itself: when one arrives it re-reads the view from the bridge, keeps it in `view`, and calls the `onView` closure we gave it. Everything else goes to the generated `EffectDispatcher`, which calls the matching `EffectHandler` method and resolves the request afterwards — never for a notification, once for a request. When a request is resolved, `Core` passes the bytes back through the bridge and loops over any **new** requests that come back. This is a direct consequence of `Command`'s async nature: a command written with `.await` points produces its next effect only after the previous one is resolved. The shell has to keep processing until the command's task finishes — and now nothing in the shell has to remember to.

What the shell writes is the `EffectHandler`. In WeatherKit that is `WeatherHandler`:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/WeatherHandler.swift}}
```

The conformance is an empty extension. `render` has a generated default, because `Core` owns it, and the other methods live beside the platform code they use (`http.swift`, `keyValue.swift`, `location.swift`, `secret.swift`, `time.swift`), all as Swift extensions on `WeatherHandler`, so they share state (like the `KeyValueStore` and the active timer list) without passing it around.

Note the `nonisolated`. The generated `EffectHandler` is `Sendable` and its requirements are not actor-isolated, but `WeatherHandler` is `@MainActor` — so the handler methods are `nonisolated` and hop to the main actor only where they touch main-actor state. URLSession, Keychain and CoreLocation work doesn't belong on the main actor anyway.

Here's the HTTP handler in full:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/http.swift}}
```

`http(_:)` is `async` and returns an `HttpResult`. That's the whole contract — the operation declares that it is answered exactly once, with an `HttpResult`, so the method signature says so and the dispatcher does the resolving. There's no request id in sight and no `resolve` call to get wrong.

The other effect handlers follow the same shape — one method per operation, returning that operation's output. The timer ones are worth a glance: `timeNotifyAfter` waits out the duration and returns the `TimerId`, while `timeClear` is a *notification* — it cancels the pending `Timer` and returns nothing. Resolving a cleared timer afterwards would hand the core an id it no longer knows about.

## Views driven by the ViewModel

`ViewStore` is a small `@Observable` class holding the latest view model; the `onView` closure in `WeatherApp` writes each new one into it, and SwiftUI views read it directly. The box exists because the generated `Core` can't be `@Observable` itself: it carries an `@available(macOS 10.15, iOS 13.0, …)` annotation, since the generated package declares no platforms, and `@Observable` needs macOS 14 / iOS 17. `@Observable` signals at the property level: when one property changes from render to render, only views attached to that property re-render. The rest of the view hierarchy stays exactly as it was, rather than rebuilding wholesale each time the model updates.

The root `ContentView` dispatches on the top-level `ViewModel` variants:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/ContentView.swift}}
```

Four lifecycle states, four views. `ActiveView` in turn dispatches on the active sub-variants (Home vs Favorites), and so on down the tree — each level of the model has a corresponding layer of view.

When the user taps a button, the view sends an event via the `CoreUpdater` that was injected into the environment at the app root. The event travels through the bridge, the core updates its state, and the `@Observable` property re-renders the view.

## Previewing without the core

Because WeatherKit never touches the FFI, previews don't need the Rust framework — or a `Core` at all. A view reads its state from `ViewStore`, so a preview constructs one with whatever `ViewModel` it wants to show, plus a `CoreUpdater.forPreview()` that swallows events:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/ViewStore.swift}}
```

Previews run as fast as regular SwiftUI previews, no FFI boundary to cross.

## What's next

That's one shell end-to-end. The core doesn't know or care what platform it's on; everything platform-specific lives here. The other shell chapters walk through the same story — booting the core, the bridge, the effect handlers, the views — in Kotlin, Rust with Leptos, and TypeScript with React.

Happy building!
