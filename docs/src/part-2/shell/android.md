# Android

Let's start with the new part, and also typically the shorter part –
implementing the capabilities.

## Capability implementation

This is what Weather's `Core.kt` looks like

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/Core.kt:core_base}}
        // ...
    }
}
```

It's slightly more complicated, but broadly the same as the Counter's core.
We have an extra logger which is not really important for us, and we
also hold on to a `KeyValueStore`, which is the storage for the key-value
implementation. The dependencies (`HttpClient`, `LocationManager`, `KeyValueStore`)
are injected via the constructor using Koin DI.

The `processRequest` method handles each effect type:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/Core.kt:process_request}}
```

We get a Request, and do an exhaustive match on what the requested effect is. In Kotlin
we have sealed classes, so we can use a `when` expression to also destructure the
operation requested.

We can have a look at what the HTTP branch does:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/Core.kt:http}}
```

This delegates to `handleHttpEffect`, which does the actual work:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/Core.kt:handle_http}}
```

We launch a coroutine to run this job off the main thread, then use the
`httpClient.request()` call to run the request.

Then it takes the response, serializes it and passes it to `core.resolve` via `resolveAndHandleEffects`, which
**returns more effect requests**. This is perhaps unexpected, but it's the direct
consequence of the `Command`s async nature. There can easily be a command which
does something along the lines of:

```rust,noplayground
Command::new(|ctx| {
    let http_req = Http::get(url).expect_json<Counter>().build().into_future(ctx);
    let resp = http_req.await; // effect 1

    let counter = resp.map(|result| match result {
        Ok(mut response) => match response.take_body() {
            Some(counter) => {
                Ok(results)
            }
            None => Err(ApiError::ParseError),
        },
        Err(_) => Err(ApiError::NetworkError),
    });

    let _ = KeyValue::set(COUNTER, counter).into_future(ctx).await // effect 2

    // ...

    ctx.send_event(Event::Done);
})
```

Once we resolve the http request at the `.await` point marked "effect 1", this future can
proceed and make a `KeyValue` request at the "effect 2" `.await` point. So on the
shell end, we need to be able to respond appropriately.

What we do is loop through those effect requests (there could easily be multiple requests
at once), go through them and recurse - call `processRequest` again to handle it.

This is what `resolveAndHandleEffects` looks like:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/Core.kt:resolve}}
```

Just for completeness, this is what the `request` method on `HttpClient` looks like:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/core/HttpClient.kt:request}}
```

Not that interesting, it's a wrapper around Ktor's `HttpClient` which takes and
returns the generated `HttpRequest` and `HttpResponse`, originally defined in Rust by
`crux_http`.

The pattern repeats similarly for key-value store and the location capability.

## User interface and navigation

It's worth looking at how Weather handles the Workflow navigation in Jetpack Compose.

As in the Counter example, the Weather's core has a `StateFlow<ViewModel>`
which we can collect with `collectAsState()` in the composables.

Here's the root content view:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/MainActivity.kt:content_view}}
```

Thanks to the declarative nature of Jetpack Compose, we can show the view we need to,
depending on the workflow, and pass the core down. We use `AnimatedContent` with
a `when` block to switch between screens based on the current workflow state, and
a `BackHandler` to navigate back when the user presses the system back button.

We could do this differently - core could stay in the root view and we could pass
an `update` callback in a `CompositionLocal`, and just the appropriate section of the
view model to each view, it's up to you how you want to go about it.

Let's look at the HomeScreen as well, just to complete the picture:

```kotlin
{{#include ../../../../examples/weather/android/app/src/main/java/com/crux/example/weather/ui/home/HomeScreen.kt:home_screen}}
```

It uses `koinViewModel` to obtain its view model, collects the UI state,
draws the weather cards for each favorite using a `HorizontalPager`, and adds
a toolbar with an `IconButton`, which when tapped calls `onShowFavorites`
with the Kotlin equivalent of the `.navigate` event we saw earlier.

This is quite a simple navigation setup in that it is a static set of screens
we're managing. Sometimes a more dynamic navigation is necessary, but
Jetpack Compose Navigation with `NavHost` supports quite complex scenarios in
a declarative fashion, so the general principle of naively projecting the view model
into the user interface broadly works even there.

There isn't much more to it, the rest of the app is rinse and repeat. It is
relatively rare to implement a new capability, so most of the work is in finessing
the user interface. Crux tends to work reasonably well with Compose previews as well
so you can typically avoid the Emulator or device for the inner development loop.

## What's next

Congratulations! You know now all you will likely need to build Crux apps. The
following parts of the book will cover advanced topics, other support platforms,
and internals of Crux, should you be interested in how things work.

Happy building!
