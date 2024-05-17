# Message interface between core and shell

So far in this book, we've been taking the perspective of being inside the core
looking out. It feels like it's now time to be in the shell, looking in.

```admonish note
Interestingly, we think this is also the way to approach building apps with Crux. For any one feature, start in the middle and get your behaviour established first. Write the tests without the UI and the other side-effects in the way. Give yourself maximum confidence that the feature works _exactly_ as you expect before you muddy the water with UI components, and their look and feel.
```

OK, let's talk about the shell.

The shell only has two responsibilities:

1. Laying out the UI components
2. Supporting the app's capabilities

We'll look at these separately. But first let's remind ourselves of how we
interact with the core (now would be a good time to read
[Shared core and types](../getting_started/core.md) if you haven't already).

## The message protocol

The interface is message based, and uses serialization to pass data back and
forth. The core exports the types for all the data so that it can be used and
created on the shell side with safety.

An `Event` can be passed in directly, as-is. Processing of `Effect`s is a little
more complicated, because the core needs to be able to pair the outcomes of the
effects with the original capability call, so it can return them to the right
caller. To do that, effects are wrapped in a `Request`, which tags them with an
Id. To respond, the same Id needs to be passed back in.

Requests from the core are emitted serialized, and need to be deserialized
first. Both events and effect outputs need to be serialized before being passed
back to the core.

```admonish warning title="Sharp edge"
It is likely that this will become an implementation detail and instead, Crux will provide a more ergonomic shell-side API for the interaction, hiding both the EffectId pairing and the serialization (and allowing us to iterate on the FFI implementation which, we think, could work better).
```

## The core interface

There are only three touch-points with the core.

```rust
pub fn process_event(data: &[u8]) -> Vec<u8> { todo!() }
pub fn handle_response(id: u32, data: &[u8]) -> Vec<u8> { todo!() }
pub fn view() -> Vec<u8> { todo!() }
```

The `process_event` function takes a serialized `Event` (from a UI interaction)
and returns a serialized vector of `Request`s that the shell can dispatch to the
relevant capability's shell-side code (see the section below on how the shell
handles capabilities).

The `handle_response` function, used to return capability output back into the
core, is similar to `process_event` except that it also takes a `id`, which
ties the output (for example a HTTP response) being submitted with it's original
`Effect` which started it (and the corresponding request which the core wrapped
it in).

The `view` function simply retrieves the serialized view model (to which the UI
is bound) and is called by the shell after it receives a `Render` request. The
view model is a projection of the app's state – it reflects what information the
Core wants displayed on screen.

You're probably thinking, "Whoa! I just see slices and vectors of bytes, where's
the type safety?". Well, the answer is that we also generate all the types that
pass through the bridge, for each language, along with serialization and
deserialization helpers. This is done by the `serde-generate` crate (see the
section on
[Create the shared types crate](../getting_started/core.md#create-the-shared-types-crate)).

```admonish warning title="Sharp edge"
For now we have to manually invoke the serialization code in the shell. At some point this may be abstracted away.
```

In this code snippet from the
[Counter example](https://github.com/redbadger/crux/blob/master/examples/counter/Android/app/src/main/java/com/example/counter/Core.kt),
notice that we call `processEvent` and `handleResponse` on the core depending on
whether we received an `Event` from the UI or from a capability, respectively.
Regardless of which core function we call, we get back a bunch of requests,
which we can iterate through and do the relevant thing (the following snippet
triggers a render of the UI, or makes an HTTP call, or launches a task to wait
for Server Sent Events, depending on what the core requested):

```kotlin
class Core : androidx.lifecycle.ViewModel() {
    var view: ViewModel? by mutableStateOf(null)
        private set

    private val httpClient = HttpClient(CIO)
    private val sseClient = HttpClient(CIO) {
        engine {
            endpoint {
                keepAliveTime = 5000
                connectTimeout = 5000
                connectAttempts = 5
                requestTimeout = 0
            }
        }
    }

    init {
        viewModelScope.launch {
            update(Event.StartWatch())
        }
    }

    suspend fun update(event: Event) {
        val effects = processEvent(event.bincodeSerialize())

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }

    private suspend fun processEffect(request: Request) {
        when (val effect = request.effect) {
            is Effect.Render -> {
                this.view = ViewModel.bincodeDeserialize(view())
            }

            is Effect.Http -> {
                val response = requestHttp(httpClient, effect.value)

                val effects =
                    handleResponse(
                        request.id.toUInt(),
                        HttpResult.Ok(response).bincodeSerialize()
                    )

                val requests = Requests.bincodeDeserialize(effects)
                for (request in requests) {
                    processEffect(request)
                }
            }

            is Effect.ServerSentEvents -> {
                requestSse(sseClient, effect.value) { response ->
                    val effects =
                        handleResponse(request.id.toUInt(), response.bincodeSerialize())

                    val requests = Requests.bincodeDeserialize(effects)
                    for (request in requests) {
                        processEffect(request)
                    }
                }
            }
        }
    }
}

```

## The UI components

Crux can work with any platform-specific UI library. We think it works best with
modern declarative UI frameworks such as
[SwiftUI](https://developer.apple.com/xcode/swiftui/) on iOS,
[Jetpack Compose](https://developer.android.com/jetpack/compose) on Android, and
[React](https://reactjs.org/)/[Vue](https://vuejs.org/) or a Wasm based
framework (like [Yew](https://yew.rs/)) on the web.

These frameworks are all pretty much identical. If you're familiar with one, you
can work out the others easily. In the examples on this page, we'll work in an
Android shell with Kotlin.

The components are bound to the view model, and they send events to the core.

We've already seen a "hello world" example when we were
[setting up an Android project](../getting_started/Android/android.md#create-some-ui-and-run-in-the-simulator).
Rather than print that out again here, we'll just look at how we need to enhance
it to work with Kotlin coroutines. We'll probably need to do this with any real
shell, because the update function that dispatches side effect requests from the
core will likely need to be `suspend`.

This is the `View` from the
[Counter example](https://github.com/redbadger/crux/blob/master/examples/counter/Android/app/src/main/java/com/example/counter/MainActivity.kt)
in the Crux repository.

```kotlin
@Composable
fun View(model: Model = viewModel()) {
    val coroutineScope = rememberCoroutineScope()
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
        modifier = Modifier
            .fillMaxSize()
            .padding(10.dp),
    ) {
        Text(text = "Crux Counter Example", fontSize = 30.sp, modifier = Modifier.padding(10.dp))
        Text(text = "Rust Core, Kotlin Shell (Jetpack Compose)", modifier = Modifier.padding(10.dp))
        Text(text = model.view.text, color = if(model.view.confirmed) { Color.Black } else { Color.Gray }, modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                onClick = { coroutineScope.launch { model.update(CoreMessage.Event(Evt.Decrement())) } },
                colors = ButtonDefaults.buttonColors(containerColor = Color.hsl(44F, 1F, 0.77F))
            ) { Text(text = "Decrement", color = Color.DarkGray) }
            Button(
                onClick = { coroutineScope.launch { model.update(CoreMessage.Event(Evt.Increment())) } },
                colors =
                ButtonDefaults.buttonColors(
                    containerColor = Color.hsl(348F, 0.86F, 0.61F)
                )
            ) { Text(text = "Increment", color = Color.White) }
        }
    }
}
```

Notice that the first thing we do is create a CoroutineScope that is scoped to
the lifetime of the View (i.e. will be destroyed when the `View` component is
unmounted). Then we use this scope to launch asynchronous tasks to call the
`update` method with the specific event.
`Button(onClick = { coroutineScope.launch { model.update(CoreMessage.Event(Evt.Increment())) } })`.
We can't call `update` directly, because it is `suspend` so we need to be in an
asynchronous context to do so.

## The capabilities

We want the shell to be as thin as possible, so we need to write as little
platform-specific code as we can because this work has to be duplicated for each
platform.

In general, the more domain-aligned our capabilities are, the more code we'll
write. When our capabilities are generic, and closer to the technical end of the
spectrum, we get to write the least amount of shell code to support them.
Getting the balance right can be tricky, and the right answer might be different
depending on context. Obviously the `Http` capability is very generic, but a CMS
capability, for instance, might well be much more specific.

The shell-side code for the `Http` capability can be very small. A (very) naive
implementation for Android might look like this:

```kotlin
{{#include ../../../examples/counter/Android/app/src/main/java/com/example/counter/http.kt}}
```

The shell-side code to support a capability (or "Port" in "Ports and Adapters"),
is effectively just an "Adapter" (in the same terminology) to the native APIs.
Note that it's the shell's responsibility to cater for threading and/or async
coroutine requirements (so the above Kotlin function is `suspend` for this
reason).

The above function can then be called by the shell when an effect is emitted
requesting an HTTP call. It can then post the response back to the core (along
with the `id` that is used by the core to tie the response up to its original
request):

```kotlin
for (req in requests) when (val effect = req.effect) {
    is Effect.Http -> {
        val response = requestHttp(httpClient, effect.value)

        val effects =
            handleResponse(
                request.id.toUInt(),
                HttpResult.Ok(response).bincodeSerialize()
            )

        val requests = Requests.bincodeDeserialize(effects)
        for (request in requests) {
            processEffect(request)
        }
    }
    // ...
}
```
