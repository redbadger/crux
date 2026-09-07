# Android

The Android shell talks to the Rust core the same way the iOS shell does — serialise events, hand them across the FFI, deserialise effect requests, handle each effect, resolve with the response, repeat. The Kotlin and Compose idioms differ from Swift and SwiftUI, but the shape is the same.

## Booting the Core with Hilt

The Android app uses [Dagger Hilt](https://dagger.dev/hilt/) to wire up the core and its dependencies. `WeatherApplication` is annotated `@HiltAndroidApp`, which bootstraps the DI graph, and `MainActivity` is `@AndroidEntryPoint`, which lets it receive `@Inject` field injection. Handlers use constructor injection, so the module that provides the app's own dependencies is small:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/di/AppModule.kt}}
```

The only explicit provider here is `OkHttpClient`, since it isn't under our control; every handler gets `@Inject constructor(...)` and Hilt figures out the graph from there. The generated `Core` is the other thing Hilt can't construct on its own — it has no `@Inject` constructor — so a second module builds it:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/di/CoreModule.kt:start}}
```

`Core` takes the three things a shell supplies: a `CoreBridge` (bound to `LiveBridge` in the same module), an `EffectHandler`, and a `CoroutineScope` on the main dispatcher. The `.also` sends the same `Event.Start` we saw in chapter 3 the moment the core exists — it fetches the API key and favourites before anything is drawn.

The handler is `WeatherHandler`, which takes five injected dependencies — one per capability that needs a real-world implementation: `HttpHandler` (OkHttp), `LocationHandler` (Fused Location Provider + permission flow), `KeyValueHandler` (DataStore-backed), `SecretStore` (AndroidKeyStore-backed), and `TimeHandler` (coroutine timers).

One thing to flag upfront: the word "ViewModel" shows up in two senses on Android. Crux's own `ViewModel` is the state projection produced by the core — what the UI ultimately consumes. Android's `androidx.lifecycle.ViewModel` is the lifecycle-aware class that survives configuration changes. The per-screen Android VMs (`HomeViewModel`, `FavoritesViewModel`, `OnboardViewModel`) sit between them: they observe a flow of Crux view models from `Core` and map each one to a Compose-friendly UI state. All three are `@HiltViewModel @Inject constructor(...)`.

## The FFI bridge

`LiveBridge` is the generated `CoreBridge` interface, implemented over BoltFFI's `CoreFfi`:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/LiveBridge.kt}}
```

Bytes in, bytes out. The generated `Core` does the serializing, so this is the only class that knows `CoreFfi` exists. The view layer observes `core.view`, a `StateFlow<ViewModel>` — a Kotlin coroutines type that always holds a current value, and conflates on equality: collectors are only notified if the new value differs from the previous one. That property keeps identical renders from rippling downstream.

One build detail: `Core` uses `StateFlow` and `launch`, so the `shared` Gradle module that compiles the generated sources declares `kotlinx-coroutines-core` as an `api` dependency. The generated `build.gradle.kts` lists it too, but this project pulls the generated sources in directly with `srcDirs` and manages versions in `libs.versions.toml`.

## Handling effects

The loop is the generated `Core`:

```kotlin
class Core(bridge: CoreBridge, handler: EffectHandler, scope: CoroutineScope) {
    val view: StateFlow<ViewModel>
    fun update(event: Event)
    fun process(requests: List<Request>)
    fun process(bytes: ByteArray)
}
```

`Core` handles `Render` itself — it re-reads the view from the bridge and publishes it on `view` — and hands everything else to the generated `EffectDispatcher`, which calls the matching `EffectHandler` method and resolves the request with whatever it returns. There's no `when` over the sealed `Effect` class anywhere. Each request gets its own coroutine on the scope you pass, because `dispatch` is `suspend`: a debounce timer suspends for its whole duration, and the requests queued behind it must not wait for it. When a request is resolved, `Core` passes the bytes through the bridge and **recurses** with the new requests that come back — same reason as in the iOS chapter: `Command` is async, and a command with multiple `.await` points produces its next effect only after the previous one is resolved. The shell has to keep looping, and now nothing in the shell has to remember to.

What the shell writes is the handler, `WeatherHandler`, whose methods are one-liners that delegate to the injected handlers. HTTP, for example:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/WeatherHandler.kt:handle_http}}
```

`suspend fun http(operation: HttpRequest): HttpResult` — the whole contract in one signature. The operation declares that it is answered exactly once with an `HttpResult`, so the method returns one, and nothing in `WeatherHandler` calls `resolve`. There is no `render` override either, because `Core` owns it; `timeClear` is the one notification here, so it returns `Unit` and is never resolved.

`httpHandler.request(...)` is the `suspend` function that wraps OkHttp:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/core/HttpHandler.kt:request}}
```

Because the handler interface owns the response types, the per-capability handlers lost their `when` blocks too: `KeyValueHandler.get(operation: Get): ValueResult` takes exactly the operation it serves and returns exactly its output, rather than matching a wide operation enum and constructing a matching response variant.

```admonish note title="Kotlin name collisions"
`crux_kv`'s `Set` operation generates a Kotlin class called `Set`, which
collides with `kotlin.collections.Set`. `WeatherHandler.kt` imports it as
`import com.crux.example.weather.Set as KeyValueSet`. The same collision
appears in Swift and TypeScript, and the same fix — an alias at the import —
works there.
```

## Views driven by the Crux view model

The generated `Core` exposes the current view model as `view: StateFlow<ViewModel>`, so Compose can collect it with `collectAsState()` and recompose when it changes; `core/Projections.kt` adds per-screen extension functions (`core.homeViewModel()` and friends) that narrow it to one branch. The root of the view tree lives in `MainActivity.onCreate`:

```kotlin
{{#include ../../../../examples/weather/Android/app/src/main/java/com/crux/example/weather/MainActivity.kt:content_view}}
```

`AnimatedContent` cross-fades between screens as the lifecycle state changes. A `when` block dispatches on the top-level `ViewModel` variants, and `ActiveViewModel` gets a nested `when` for Home vs Favorites.

The individual screens (`HomeScreen`, `FavoritesScreen`, `OnboardScreen`) don't take the Crux view model directly — they get a per-screen Android `ViewModel` via `hiltViewModel()`, which owns a `UiStateMapper` that transforms the Crux data into a Compose-friendly `UiState`. This is standard Android MVVM and keeps the Compose layer free of Crux-specific types.

Two things keep that loop efficient. `StateFlow` suppresses equal emissions, so if a screen's mapper produces a `UiState` that equals the previous one, the flow doesn't emit at all. When it does emit, Compose's recomposition is equality-based — composables whose inputs haven't changed are skipped. The practical effect is the same as iOS's `@Observable`: a small change in the Crux model triggers a small recomposition, not a sweep of the whole tree.

## What's next

That's the Android shell. Structure-wise it mirrors iOS: events go in, effects come out, the view layer collects the view model. The rest of the app is screens and view models — standard Compose work.

Happy building!
