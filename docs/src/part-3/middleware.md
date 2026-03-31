# Middleware

Middleware is a relatively new, and somewhat advanced feature for split
effect handling, i.e. handling some effects in the shell, and some
still in the core, but outside the app's state loop.

Middleware can be useful when you have an existing 3rd party library written in Rust
which you want to use, but it isn't written in a sans-I/O way with managed
effects or otherwise isn't compatible with Crux. This is sadly most libraries
with side effects.

It is quite likely most apps will never need to use middleware. Before reaching for middleware,
we encourage you to consider:

- Implementing the side-effect in each Shell using native, platform SDKs. Shared libraries give a
  productivity boost at first, but for the same reason Crux uses Capabilities, they can't always
  be the best platform citizens, and often rely on very low-level system APIs which compromise the
  experience, don't collaborate well with platform security measures, etc.
- Moving coordination logic from the Rust implementation into a custom capability in the core
  and implementing it on top of lower level capabilities, e.g. HTTP. This would be the case for
  HTTP API SDK type libraries, but may well not be practical at first

Only if neither of these is a good option, reach for a middleware. The cost of using it is
that the effect handling becomes less straightforward, which may cause some headaches
debugging effect ordering, etc.

We are also still learning how middleware operates in the wild, and the API may change more
than the rest of Crux tends to.

All that said, the feature is used in production with success today and should work well.

## How it works

Middleware sits between the Core and the Shell in the effect processing pipeline. When
the app requests effects, they pass through the middleware stack on their way to the shell.
A middleware layer can intercept specific effect variants, handle them (performing the
side-effect in Rust), and resolve the request — all without the shell ever seeing that effect.
Effects the middleware doesn't handle pass through to the shell as normal.

We'll walk through the
[counter-middleware](https://github.com/redbadger/crux/tree/master/examples/counter-middleware)
example to see how this works in practice. This example is a counter app that has a "random"
button — when pressed, the counter changes by a random amount. The random number generation
is handled by a middleware, rather than by the shell.

## Defining the operation

First, we need an `Operation` type that describes the request and its output. This is the
same as defining a capability's protocol — a request type and a response type:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/capabilities/mod.rs:9:17}}
```

The `RandomNumberRequest` carries the range (min, max), and `RandomNumber` carries the result.
The `Operation` impl connects them so that Crux knows a `RandomNumberRequest` produces a
`RandomNumber`.

The app uses this operation as one variant of its `Effect` enum:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/app.rs:62:69}}
```

And the app can request a random number using `Command::request_from_shell`, just as it
would for any shell-handled effect:

```rust,no_run,noplayground
            Event::Random => Command::request_from_shell(RandomNumberRequest(-5, 5))
                .map(|out| out.0)
                .then_send(Event::UpdateBy),
```

The app doesn't know or care that this effect will be intercepted by middleware — it just
requests the effect and handles the response.

## Implementing `EffectMiddleware`

The `EffectMiddleware` trait is how you tell Crux what to do when it encounters a specific
effect. You implement `try_process_effect`, which receives the operation and an
`EffectResolver` that you use to send back the result.

Here's the `RngMiddleware` from the example:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/middleware.rs}}
```

A few things to note:

- The `type Op` associated type tells Crux which operation this middleware handles
  (`RandomNumberRequest` in this case).
- `try_process_effect` receives the operation and an `EffectResolver`. You must call
  `resolver.resolve(output)` with the result when the work is done.
- The processing happens on a background thread. This is important — the middleware
  must not block the caller of `process_event`. On native targets this typically means
  spawning a thread; on WASM it means an async task (e.g. `spawn_local`).
- The background thread pattern shown here (a persistent worker with a channel) is a
  good approach when the middleware holds state (like the RNG seed). For stateless work,
  you could simply spawn a thread per request.

## Wiring it up

The middleware is composed with the Core in the FFI module, where you build the bridge
between the core and the shell. Here's the key part from the uniffi (native) FFI setup:

```rust,no_run,noplayground
        pub fn new(shell: Arc<dyn CruxShell>) -> Self {
            let core = Core::<Counter>::new()
                .handle_effects_using(RngMiddleware::new())
                .map_effect::<Effect>()
                .bridge::<BincodeFfiFormat>(move |effect_bytes| match effect_bytes {
                    Ok(effect) => shell.process_effects(effect),
                    Err(e) => panic!("{e}"),
                });

            Self { core }
        }
```

This reads bottom-to-top as a pipeline:

1. **`Core::<Counter>::new()`** — creates the core, which produces the app's full `Effect`
   enum (including the `Random` variant).
2. **`.handle_effects_using(RngMiddleware::new())`** — wraps the core with the RNG middleware.
   Any `Random` effects are intercepted and handled here; all other effects pass through.
3. **`.map_effect::<Effect>()`** — narrows the effect type. Since the middleware has consumed
   all `Random` effects, the shell will never see them. This step converts to a _new_ `Effect`
   enum that doesn't include the `Random` variant, so your shell code doesn't need an
   unreachable branch.
4. **`.bridge::<BincodeFfiFormat>(...)`** — creates the FFI bridge as usual.

### The narrowed effect type

The FFI module defines its own `Effect` enum without the `Random` variant:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/ffi.rs:16:21}}
```

And a `From` implementation to convert from the app's full effect type:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/ffi.rs:23:32}}
```

The `Random` arm panics because it should never be reached — the middleware handles all
`Random` effects before they get here.

## Testing

The app can be tested exactly the same way as any other Crux app — the middleware is not
involved in unit tests. You test the app's `update` function directly, treating `Random`
as a normal effect:

```rust,no_run,noplayground
{{#include ../../../examples/counter-middleware/shared/src/app.rs:509:525}}
```

This is one of the nice properties of middleware: the app logic remains pure and testable,
and the middleware is a separate concern that's composed at the FFI boundary.

## Summary

To add a middleware to your app:

1. **Define an `Operation`** — a request type and output type, just like a capability protocol.
2. **Implement `EffectMiddleware`** — handle the operation and resolve the result, typically
   on a background thread.
3. **Wire it up** — use `.handle_effects_using()` in your FFI setup to intercept the effects,
   and optionally `.map_effect()` to narrow the effect type for the shell.

For the full API reference, see the
[middleware module docs](https://docs.rs/crux_core/latest/crux_core/middleware/index.html).
