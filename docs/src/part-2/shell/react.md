# React

Let's start with the new part, and also typically the shorter part –
implementing the capabilities.

## Capability implementation

This is what Weather's `core.ts` looks like

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/core.ts:core_base}}
  // ...
}
```

It's slightly more complicated, but broadly the same as the Counter's core.
We wrap the `CoreFFI` (loaded via WASM) and hold on to a React `setState`
callback, which we use to update the view model whenever the core asks us to
render.

We've truncated the `resolve` method, because it's fairly long, but the basic
structure is this:

```typescript
  async resolve(id: number, effect: Effect) {
    switch (effect.constructor) {
      case EffectVariantRender:
        // ...

      case EffectVariantHttp:
        // ...

      case EffectVariantKeyValue:
        // ...

      case EffectVariantLocation:
        // ...
    }
  }
```

We get a Request, and do a switch on what the requested effect's constructor is
to determine the type. In TypeScript we use `instanceof`-style constructor
checks, so we can also cast and destructure the operation requested.

We can have a look at what the HTTP branch does:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/core.ts:http}}
```

This delegates to `http.request()`, which does the actual work, and then calls
`this.respond()` with the result:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/core.ts:respond}}
```

We use `async`/`await` to run the HTTP request, then take the response,
serialize it and pass it to `core.resolve` via `respond`, which
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
at once), go through them and recurse—call `resolve` again to handle each one.

Just for completeness, this is what `http.ts` looks like:

```typescript
{{#include ../../../../examples/weather/web-nextjs/src/app/http.ts}}
```

Not that interesting, it's a wrapper around the browser's native `fetch` API which takes and
returns the generated `HttpRequest` and `HttpResponse`, originally defined in Rust by
`crux_http`.

The pattern repeats similarly for key-value store and the location capability.

## User interface and navigation

It's worth looking at how Weather handles the Workflow navigation in React.

As in the Counter example, the Weather's core holds a `ViewModel` which we
store in React state via `useState`, so the component re-renders whenever the
core asks us to.

Here's the root component:

```tsx
{{#include ../../../../examples/weather/web-nextjs/src/app/page.tsx:content_view}}
```

We initialize the WASM core inside a `useEffect` that runs once, create a
`Core` instance with the `setView` callback, and immediately dispatch the
initial `Show` event to kick things off.

Thanks to the declarative nature of React, we can show the view we need to,
depending on the workflow, using `instanceof` checks on the workflow variant.
Each branch renders the appropriate component and passes the core ref down.

We could do this differently—core could stay in the root component and we
could pass an `update` callback via React context, and just the appropriate
section of the view model to each component. You could also use React Router
for navigation. It's up to you how you want to go about it.

Let's look at the HomeView as well, just to complete the picture:

```tsx
{{#include ../../../../examples/weather/web-nextjs/src/app/page.tsx:home_view}}
```

It simply caters for the possible situations in the view model—checking
whether `cod === 200` to decide if weather data has loaded—draws the
weather cards with a grid of details, and adds a "Favorites" button which
when clicked calls `core.current?.update` with the TypeScript equivalent of
the `.navigate` event we saw earlier in the core.

This is quite a simple navigation setup in that it is a static set of screens
we're managing. Sometimes a more dynamic navigation is necessary, but
React Router or similar libraries support quite complex scenarios in a
declarative fashion, so the general principle of naively projecting the view
model into the user interface broadly works even there.

There isn't much more to it, the rest of the app is rinse and repeat. It is
relatively rare to implement a new capability, so most of the work is in finessing
the user interface. Crux tends to work reasonably well with hot module reloading
so you can typically avoid full page reloads for the inner development loop.

## What's next

Congratulations! You know now all you will likely need to build Crux apps. The
following parts of the book will cover advanced topics, other support platforms,
and internals of Crux, should you be interested in how things work.

Happy building!
