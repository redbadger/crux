# Android

The Android shell talks to the Rust core the same way the iOS shell does — serialise events, hand them across the FFI, deserialise effect requests, handle each effect, resolve with the response, repeat. The Kotlin and Compose idioms differ from Swift and SwiftUI, but the shape is the same.

## Booting the Core with Hilt

The Android app uses [Dagger Hilt](https://dagger.dev/hilt/) to wire up the core and its dependencies. `WeatherApplication` is annotated `@HiltAndroidApp`, which bootstraps the DI graph, and `MainActivity` is `@AndroidEntryPoint`, which lets it receive `@Inject` field injection. Handlers and the core itself use constructor injection — so the Hilt module is small:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/di/AppModule.kt}}
```

The only explicit provider is `OkHttpClient`, since it isn't under our control. `Core` and every handler get `@Inject constructor(...)` — Hilt figures out the graph from there.

`Core` itself takes five injected dependencies — one per capability that needs a real-world implementation: `HttpHandler` (OkHttp), `LocationHandler` (Fused Location Provider + permission flow), `KeyValueHandler` (DataStore-backed), `SecretStore` (AndroidKeyStore-backed), and `TimeHandler` (coroutine timers).

One thing to flag upfront: the word "ViewModel" shows up in two senses on Android. Crux's own `ViewModel` is the state projection produced by the core — what the UI ultimately consumes. Android's `androidx.lifecycle.ViewModel` is the lifecycle-aware class that survives configuration changes. The per-screen Android VMs (`HomeViewModel`, `FavoritesViewModel`, `OnboardViewModel`) sit between them: they observe a flow of Crux view models from `Core` and map each one to a Compose-friendly UI state. All three are `@HiltViewModel @Inject constructor(...)`.

`Core` kicks the lifecycle off right in its `init` block:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:start}}
```

The same `Event.Start` we saw in chapter 3 — the moment the core is constructed, it fetches the API key and favourites.

## The FFI bridge

Kotlin doesn't have a separate bridge file like Swift's `LiveBridge`; the bridging is inline in `Core.kt`. Here's the top of the class with the FFI instance and the flow the view layer observes:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:core_base}}
```

`update(event)` serialises the event with bincode, calls `coreFfi.update(...)`, and hands the resulting bytes to `handleEffects`. The Crux view-model flow (`_viewModel`) is a `MutableStateFlow<ViewModel>` — a Kotlin coroutines type that always holds a current value, and conflates on equality: when you set the flow's `.value`, collectors are only notified if the new value differs from the previous one. That property keeps identical renders from rippling downstream.

## Handling effects

`Core` implements the generated `EffectHandler` interface, and every request goes to the generated `EffectDispatcher`:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:process_request}}
```

There's no `when` over the sealed `Effect` class any more — the dispatcher does that, calls the matching handler method, and resolves the request with whatever it returns. Each request gets its own coroutine, because `dispatch` is `suspend`: a debounce timer suspends for its whole duration, and the requests queued behind it must not wait for it.

The handler methods are one-liners that delegate to the injected handlers. HTTP, for example:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:handle_http}}
```

`suspend fun http(operation: HttpRequest): HttpResult` — the whole contract in one signature. The operation declares that it is answered exactly once with an `HttpResult`, so the method returns one, and nothing in `Core` calls `resolve`. Compare `override fun render(operation: RenderOperation) = render()`: a notification, so it returns `Unit` and is never resolved. `timeClear` is the other notification.

`httpHandler.request(...)` is the `suspend` function that wraps OkHttp:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/HttpHandler.kt:request}}
```

Because the handler interface owns the response types, the per-capability handlers lost their `when` blocks too: `KeyValueHandler.get(operation: Get): ValueResult` takes exactly the operation it serves and returns exactly its output, rather than matching a wide operation enum and constructing a matching response variant.

The dispatcher resolves through the callback `Core` gave it, which calls `coreFfi.resolve(...)` and then **recurses** through `handleEffects` with the new effect requests:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/Core.kt:resolve}}
```

Same reason as in the iOS chapter: `Command` is async, and a command with multiple `.await` points produces its next effect only after the previous one is resolved. The shell has to keep looping.

```admonish note title="Kotlin name collisions"
`crux_kv`'s `Set` operation generates a Kotlin class called `Set`, which
collides with `kotlin.collections.Set`. `Core.kt` imports it as
`import com.crux.example.weather.Set as KeyValueSet`. The same collision
appears in Swift and TypeScript, and the same fix — an alias at the import —
works there.
```

## Views driven by the Crux view model

`Core` exposes the current view model as a `StateFlow<ViewModel>`, so Compose can collect it with `collectAsState()` and recompose when it changes. The root of the view tree lives in `MainActivity.onCreate`:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/MainActivity.kt:content_view}}
```

`AnimatedContent` cross-fades between screens as the lifecycle state changes. A `when` block dispatches on the top-level `ViewModel` variants, and `ActiveViewModel` gets a nested `when` for Home vs Favorites.

The individual screens (`HomeScreen`, `FavoritesScreen`, `OnboardScreen`) don't take the Crux view model directly — they get a per-screen Android `ViewModel` via `hiltViewModel()`, which owns a `UiStateMapper` that transforms the Crux data into a Compose-friendly `UiState`. This is standard Android MVVM and keeps the Compose layer free of Crux-specific types.

Two things keep that loop efficient. `StateFlow` suppresses equal emissions, so if a screen's mapper produces a `UiState` that equals the previous one, the flow doesn't emit at all. When it does emit, Compose's recomposition is equality-based — composables whose inputs haven't changed are skipped. The practical effect is the same as iOS's `@Observable`: a small change in the Crux model triggers a small recomposition, not a sweep of the whole tree.

## What's next

That's the Android shell. Structure-wise it mirrors iOS: events go in, effects come out, the view layer collects the view model. The rest of the app is screens and view models — standard Compose work.

Happy building!
