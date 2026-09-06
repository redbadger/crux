# iOS/macOS

This is the first of the shell chapters. We'll walk through how the Swift side talks to the Rust core, how each effect gets carried out, and how the views consume the view model. The other shell chapters follow the same structure in their own idioms.

## The WeatherKit package

The Apple shell is split into two Swift targets:

- **`WeatherApp`** (the app target) — just a few files: the `@main` struct, the `LiveBridge` that talks to Rust, and `ContentView` as the root view.
- **`WeatherKit`** (a local Swift Package) — everything else: `Core`, every effect handler, every screen.

The split exists because building Swift is much faster than rebuilding the whole Rust framework, and SPM gives you the kind of iteration loop you'd expect from `cargo`. When you're tweaking a view, you only recompile the package. When you're iterating on effect handlers, same — the Rust library (and the Swift bindings it emits) only recompile when the core changes.

Everything in WeatherKit is written against a `CoreBridge` protocol rather than talking to the Rust FFI directly. That's what lets SwiftUI previews construct a `Core` with a `FakeBridge`; they don't need the Rust framework loaded. More on that at the end.

## Booting the Core

Here's the app entry point:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/WeatherApp.swift:start}}
```

Five lines: construct the bridge, build the `Core`, bind it to SwiftUI state, wire up an `updater`, and send `Event::Start` to kick the lifecycle. After that, the core starts fetching the API key and favourites — everything we described in chapter 3.

## The FFI bridge

`LiveBridge` is the thin Swift type that carries events and effect responses across the FFI boundary:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/LiveBridge.swift}}
```

Three responsibilities:

- `processEvent(_:)` serialises a Swift `Event` with bincode, calls `CoreFfi.update(data:)`, and deserialises the returned effect requests.
- `resolve(requestId:responseBytes:)` does the same for effect responses — and, importantly, can return further effect requests (async commands produce more effects after each resolve).
- `currentView()` deserialises the current view model.

This is the only place that knows about bincode or `CoreFfi`. Everything else in the Swift code works with Swift types.

## Handling effects

`Core` implements the generated `EffectHandler` protocol — one method per operation the app declares — and hands every request to the generated `EffectDispatcher`:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/core.swift:dispatch}}
```

No match statement at all: the dispatcher calls the right method and resolves the request afterwards. The conformance itself is a one-liner, in `handler.swift`:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/handler.swift}}
```

The rest of the methods live beside the platform code they use (`http.swift`, `keyValue.swift`, `location.swift`, `secret.swift`, `time.swift`), all as Swift extensions on `Core`, so they share state (like the `KeyValueStore` and the active timer list) without passing it around.

Note the `nonisolated`. The generated `EffectHandler` is `Sendable` and its requirements are not actor-isolated, but `Core` is `@MainActor` — so the handler methods are `nonisolated` and hop to the main actor only where they touch main-actor state. URLSession, Keychain and CoreLocation work doesn't belong on the main actor anyway.

Here's the HTTP handler in full:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/http.swift}}
```

`http(_:)` is `async` and returns an `HttpResult`. That's the whole contract — the operation declares that it is answered exactly once, with an `HttpResult`, so the method signature says so and the dispatcher does the resolving. There's no request id in sight and no `resolve` call to get wrong. Compare `render(_:)` above: a notification, so it returns nothing and is never resolved.

What the dispatcher resolves *with* is the callback `Core` gave it at construction:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/core.swift:resolve_helper}}
```

It hops to the main actor, passes the bytes to the bridge, and then loops over any **new** effect requests that came back. This is a direct consequence of `Command`'s async nature: a command written with `.await` points produces its next effect only after the previous one is resolved. The shell has to keep processing until the command's task finishes.

The other effect handlers follow the same shape — one method per operation, returning that operation's output. The timer ones are worth a glance: `timeNotifyAfter` waits out the duration and returns the `TimerId`, while `timeClear` is a *notification* — it cancels the pending `Timer` and returns nothing. Resolving a cleared timer afterwards would hand the core an id it no longer knows about.

## Views driven by the ViewModel

The `Core` class exposes its view model via `@Observable`, so SwiftUI views can read it directly. `@Observable` signals at the property level: when one property changes from render to render, only views attached to that property re-render. The rest of the view hierarchy stays exactly as it was, rather than rebuilding wholesale each time the model updates.

The root `ContentView` dispatches on the top-level `ViewModel` variants:

```swift
{{#include ../../../../examples/weather/apple/WeatherApp/ContentView.swift}}
```

Four lifecycle states, four views. `ActiveView` in turn dispatches on the active sub-variants (Home vs Favorites), and so on down the tree — each level of the model has a corresponding layer of view.

When the user taps a button, the view sends an event via the `CoreUpdater` that was injected into the environment at the app root. The event travels through the bridge, the core updates its state, and the `@Observable` property re-renders the view.

## Previewing with FakeBridge

Because WeatherKit is written against `CoreBridge`, we can construct a `Core` for SwiftUI previews without loading the Rust framework:

```swift
{{#include ../../../../examples/weather/apple/WeatherKit/Sources/WeatherKit/Core/bridge.swift}}
```

`FakeBridge` returns a static `ViewModel` and ignores everything else. Combined with the `Core.forPreviewing` helper, any view can be previewed with whatever `ViewModel` state you want — previews run as fast as regular SwiftUI previews, no FFI boundary to cross.

## What's next

That's one shell end-to-end. The core doesn't know or care what platform it's on; everything platform-specific lives here. The other shell chapters walk through the same story — booting the core, the bridge, the effect handlers, the views — in Kotlin, Rust with Leptos, and TypeScript with React.

Happy building!
