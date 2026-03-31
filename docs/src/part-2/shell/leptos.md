# Leptos

Let's start with the new part, and also typically the shorter part –
implementing the capabilities.

## Capability implementation

This is what Weather's `core.rs` looks like

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/core.rs:core_base}}
```

Because both the shell and the core are Rust, the Leptos shell is simpler than
the iOS or Android equivalents. There is no need for serialization or foreign
function interfaces—the shared types are used directly. The `Core` is an
`Rc<shared::Core<Weather>>`, and `new` and `update` are free functions rather
than methods on a class.

We've truncated the `process_effect` function, but the basic structure is this:

```rust,noplayground
    pub fn process_effect(core: &Core, effect: Effect, render: WriteSignal<ViewModel>) {
        match effect {
            Effect::Render(_) => { /* ... */ }
            Effect::Http(mut request) => { /* ... */ }
            Effect::KeyValue(mut request) => { /* ... */ }
            Effect::Location(mut request) => { /* ... */ }
        }
    }
```

In Rust we have enums, so we can pattern match and destructure the operation
requested. This is the most readable version of effect dispatch across all the
shells, since both the core and the shell speak the same language.

We can have a look at what the HTTP branch does:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/core.rs:http}}
```

We spawn a local async task via `task::spawn_local` (WASM is single-threaded,
so we use a local future rather than a multi-threaded runtime), then call
`http::request()` to perform the actual HTTP call.

Then it takes the response and passes it to `core.resolve`, which **returns
more effect requests**. This is perhaps unexpected, but it's the direct
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
at once), go through them and recurse—call `process_effect` again to handle it.

Note that unlike the iOS shell, where `resolve` returns bytes that need
deserialization, in Leptos we call `core.resolve()` directly and get `Effect`
values back—no serialization boundary to cross.

Just for completeness, this is what `http.rs` looks like:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/http.rs}}
```

Not that interesting, it's a wrapper around `gloo_net`'s HTTP client for WASM
which takes and returns the generated `HttpRequest` and `HttpResponse`,
originally defined in Rust by `crux_http`.

The pattern repeats similarly for key-value store and the location capability.

## User interface and navigation

It's worth looking at how Weather handles the Workflow navigation in Leptos.

Here's the root component:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/main.rs:content_view}}
```

We create the core with `core::new()`, then set up two pairs of reactive
signals: `(view, render)` for the view model and `(event, set_event)` for
dispatching events. An `Effect::new` watches the `event` signal and calls
`core::update` whenever it changes. The `view!` macro—Leptos's JSX-like
syntax—matches on the `WorkflowViewModel` enum to decide which child
component to render, passing the relevant data and the `set_event` writer down.

We could do this differently—the core could stay in the root component and
we could pass an update callback through Leptos context, and just the
appropriate section of the view model to each component. It's up to you how you
want to go about it.

Let's look at the HomeView as well, just to complete the picture:

```rust,noplayground
{{#include ../../../../examples/weather/web-leptos/src/main.rs:home_view}}
```

It checks whether the weather data has loaded (`cod == 200`), renders the weather
details in a grid using Bulma CSS classes, lists any favorites, and adds a
button which when clicked sets the event signal to navigate to the Favorites
screen.

This is quite a simple navigation setup in that it is a static set of screens
we're managing. Sometimes a more dynamic navigation is necessary, but Leptos
Router supports quite complex scenarios in a declarative fashion, so the
general principle of naively projecting the view model into the user interface
broadly works even there.

There isn't much more to it, the rest of the app is rinse and repeat. It is
relatively rare to implement a new capability, so most of the work is in finessing
the user interface.

## What's next

Congratulations! You know now all you will likely need to build Crux apps. The
following parts of the book will cover advanced topics, other support platforms,
and internals of Crux, should you be interested in how things work.

Happy building!
